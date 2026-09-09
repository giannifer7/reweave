#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::path::{Path, PathBuf};

use clap::Parser;
use miette::Diagnostic;
use reweave_macro::evaluator::{EvalConfig, EvalError, Evaluator};
use reweave_macro::macro_api::process_string;
use reweave_tangle::{Tangle, TangleConfig, TangleError};
use std::io::Write;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error, Diagnostic)]
enum Error {
    #[error("macro expansion failed")]
    #[diagnostic(code(reweave::macro_expand))]
    Macro {
        #[from]
        #[source]
        source: EvalError,
    },
    #[error("tangle failed")]
    #[diagnostic(code(reweave::tangle))]
    Tangle {
        #[from]
        #[source]
        source: TangleError,
    },
    #[error("I/O failed")]
    #[diagnostic(code(reweave::io))]
    Io {
        #[from]
        #[source]
        source: std::io::Error,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "reweave",
    version,
    about = "Forward-only Markdown macro expansion and noweb tangling"
)]
struct Args {
    /// Input Markdown files. Use --dir to discover files recursively.
    inputs: Vec<PathBuf>,

    /// Recursively read files under this directory.
    #[arg(long = "dir")]
    directory: Option<PathBuf>,

    /// Extension to discover in --dir mode. Repeatable.
    #[arg(long = "ext", default_value = "md")]
    extensions: Vec<String>,

    /// Output directory for @file chunks.
    #[arg(short = 'o', long = "out", default_value = ".")]
    out_dir: PathBuf,

    /// Disable macro expansion and tangle the input Markdown as-is.
    #[arg(long = "no-macro")]
    no_macro: bool,

    /// Expand macros and write the expanded Markdown to stdout without tangling.
    #[arg(long = "macro-only", conflicts_with = "no_macro")]
    macro_only: bool,

    /// Macro sigil.
    #[arg(long = "sigil", default_value = "%")]
    sigil: char,

    /// Include path for %include/%import. Repeatable.
    #[arg(short = 'I', long = "include", default_value = ".")]
    include: Vec<PathBuf>,

    /// Allow %env(NAME) to read environment variables.
    #[arg(long)]
    allow_env: bool,

    /// Optional prefix prepended to environment lookups.
    #[arg(long)]
    env_prefix: Option<String>,

    /// Define a top-level macro variable before evaluation. Form: NAME=VALUE.
    #[arg(short = 'D', long = "define")]
    define: Vec<String>,

    /// Noweb open delimiter.
    #[arg(long = "open-delim", default_value = reweave_tangle::DEFAULT_OPEN_DELIM)]
    open_delim: String,

    /// Noweb close delimiter.
    #[arg(long = "close-delim", default_value = reweave_tangle::DEFAULT_CLOSE_DELIM)]
    close_delim: String,

    /// Noweb chunk end marker.
    #[arg(long = "chunk-end", default_value = reweave_tangle::DEFAULT_CHUNK_END)]
    chunk_end: String,

    /// Comment marker accepted before chunk syntax. Repeatable.
    #[arg(long = "comment-marker", default_values_t = ["//".to_string(), "#".to_string()])]
    comment_markers: Vec<String>,

    /// External formatter applied to each output before writing, invoked as
    /// `CMD <file>`. Repeatable; runs before the change-detection comparison.
    #[arg(long = "formatter")]
    formatters: Vec<String>,

    /// Write a stamp file after a successful run (used as the ninja target).
    #[arg(long = "stamp")]
    stamp: Option<PathBuf>,

    /// Write a Makefile-style depfile covering every input that can affect
    /// the outputs: all inputs plus every resolved %include/%import target.
    #[arg(long = "depfile")]
    depfile: Option<PathBuf>,

    /// Maximum recursion depth for macro and chunk expansion.
    #[arg(long = "recursion-limit", default_value_t = reweave_core::MAX_RECURSION_DEPTH)]
    recursion_limit: usize,
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn cli_main() -> miette::Result<()> {
    let args = Args::parse();
    run(args).map_err(miette::Report::new)
}

fn run(args: Args) -> Result<(), Error> {
    let inputs = collect_inputs(&args)?;

    let mut evaluator = Evaluator::new(EvalConfig {
        sigil: args.sigil,
        include_paths: args.include.clone(),
        allow_env: args.allow_env,
        env_prefix: args.env_prefix,
        recursion_limit: args.recursion_limit,
    });
    apply_cli_defines(&mut evaluator, &args.define)?;

    if args.macro_only {
        let mut stdout = std::io::stdout().lock();
        for input in inputs {
            let text = std::fs::read_to_string(&input)?;
            let bytes = process_string(&text, Some(&input), &mut evaluator)?;
            stdout.write_all(&bytes)?;
        }
        return Ok(());
    }

    let expanded = expand_inputs(&inputs, &mut evaluator, &args.include, args.no_macro)?;

    let mut tangle = Tangle::new(TangleConfig {
        open_delim: args.open_delim,
        close_delim: args.close_delim,
        chunk_end: args.chunk_end,
        comment_markers: args.comment_markers,
        strict_undefined: true,
        recursion_limit: args.recursion_limit,
        formatters: args.formatters,
    });

    for (input, text) in &expanded.mains {
        tangle.read(text, &input.to_string_lossy());
    }

    let written = tangle.write_files(&args.out_dir)?;

    if let Some(depfile) = &args.depfile {
        write_depfile(depfile, args.stamp.as_deref(), &inputs, &expanded.includes)?;
    }
    if let Some(stamp) = &args.stamp {
        write_stamp(stamp, &written)?;
    }
    Ok(())
}

/// Result of the macro-expansion phase.
#[derive(Debug)]
struct ExpandedInputs {
    /// Main documents (inputs not claimed as an include) with expanded text.
    mains: Vec<(PathBuf, String)>,
    /// Every resolved %include/%import target (for depfile emission).
    includes: Vec<PathBuf>,
}

/// Macro-expand all inputs, skipping `%include`d fragments.
///
/// Fragments cannot be expanded standalone (they may use macros defined in
/// the main document) and must not be tangled standalone (their chunks are
/// already spliced into the main document's expansion, so a second pass would
/// trip the duplicate-@file check). The evaluator is the only classification
/// authority: a file is a fragment exactly when some other input included it.
///
/// Scheduling is minimal by construction: a static scan of *literal*
/// `%include(path)` args orders inputs main-document-first, so in the common
/// case every file is evaluated exactly once (main documents directly,
/// fragments via the include splice). The hint only orders work; it never
/// classifies. Files the hint misses (macro-computed include args) fall back
/// to deferral passes: an input that fails to expand is retried after the
/// other inputs have had a chance to claim it as an include, and a pass with
/// no progress means the failures are genuine.
fn expand_inputs(
    inputs: &[PathBuf],
    evaluator: &mut Evaluator,
    include_paths: &[PathBuf],
    no_macro: bool,
) -> Result<ExpandedInputs, Error> {
    let mut expansions: Vec<(PathBuf, String)> = Vec::new();
    let mut fragments: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut includes: Vec<PathBuf> = Vec::new();

    if no_macro {
        for input in inputs {
            expansions.push((input.clone(), std::fs::read_to_string(input)?));
        }
        return Ok(ExpandedInputs {
            mains: expansions,
            includes,
        });
    }

    // Canonical path of every input, to recognize resolved include targets.
    let mut canonical_inputs: std::collections::HashMap<PathBuf, PathBuf> =
        std::collections::HashMap::new();
    for p in inputs {
        if let Ok(c) = p.canonicalize() {
            canonical_inputs.insert(c, p.clone());
        }
    }

    let mut pending = schedule(inputs, include_paths, &canonical_inputs);
    loop {
        let mut next_pending = Vec::new();
        let mut first_err = None;
        let mut progress = false;
        for input in pending {
            if fragments.contains(&input) {
                // Claimed as an include by an earlier main document.
                progress = true;
                continue;
            }
            let text = std::fs::read_to_string(&input)?;
            match process_string(&text, Some(&input), evaluator) {
                Ok(bytes) => {
                    for p in evaluator.drain_included_paths() {
                        includes.push(p.clone());
                        if let Ok(canon) = p.canonicalize()
                            && let Some(orig) = canonical_inputs.get(&canon)
                        {
                            fragments.insert(orig.clone());
                        }
                    }
                    expansions.push((input, String::from_utf8_lossy(&bytes).into_owned()));
                    progress = true;
                }
                Err(e) => {
                    first_err.get_or_insert(e);
                    next_pending.push(input);
                }
            }
        }
        if next_pending.is_empty() {
            break;
        }
        if !progress {
            return Err(Error::Macro {
                source: first_err.expect("pending inputs imply an error"),
            });
        }
        pending = next_pending;
    }

    // An input expanded before being claimed must still not be tangled.
    expansions.retain(|(p, _)| !fragments.contains(p));
    Ok(ExpandedInputs {
        mains: expansions,
        includes,
    })
}

/// Order inputs main-document-first using literal `%include`/`%import`
/// arguments as scheduling hints. Files not connected by hints keep their
/// relative input order. The evaluator remains the classification authority;
/// this only avoids expanding a fragment before it is claimed.
fn schedule(
    inputs: &[PathBuf],
    include_paths: &[PathBuf],
    canonical_inputs: &std::collections::HashMap<PathBuf, PathBuf>,
) -> Vec<PathBuf> {
    // Map each input to the inputs it statically appears to include.
    let mut hinted: std::collections::HashMap<&Path, Vec<PathBuf>> =
        std::collections::HashMap::new();
    for input in inputs {
        let Ok(text) = std::fs::read_to_string(input) else {
            continue;
        };
        let mut targets: Vec<PathBuf> = Vec::new();
        for hint in include_hints(&text) {
            if let Some(canon) = resolve_hint(&hint, include_paths)
                && let Some(orig) = canonical_inputs.get(&canon)
            {
                targets.push(orig.clone());
            }
        }
        if !targets.is_empty() {
            hinted.insert(input.as_path(), targets);
        }
    }

    // Pre-order DFS: emit a main document before the fragments it includes.
    // Seed from documents with hinted includes first so claims exist before
    // their fragments are considered; unconnected files keep input order.
    let mut order = Vec::with_capacity(inputs.len());
    let mut visited = std::collections::HashSet::new();
    for input in inputs {
        if hinted.contains_key(input.as_path()) {
            visit(input, &hinted, &mut visited, &mut order);
        }
    }
    for input in inputs {
        visit(input, &hinted, &mut visited, &mut order);
    }
    order
}

fn visit(
    input: &Path,
    hinted: &std::collections::HashMap<&Path, Vec<PathBuf>>,
    visited: &mut std::collections::HashSet<PathBuf>,
    order: &mut Vec<PathBuf>,
) {
    if !visited.insert(input.to_path_buf()) {
        return;
    }
    order.push(input.to_path_buf());
    if let Some(targets) = hinted.get(input) {
        for target in targets {
            visit(target, hinted, visited, order);
        }
    }
}

/// Literal include arguments, e.g. `%include(src/frag.md)`. Arguments
/// containing macro syntax are not hints — the evaluator resolves those.
#[cfg_attr(coverage_nightly, coverage(off))]
fn include_hints(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r"%(?:include|import)\(\s*([A-Za-z0-9_./-]+\.md)\s*\)")
        .expect("valid hint regex");
    re.captures_iter(text)
        .map(|c| c[1].to_string())
        .collect()
}

/// Resolve a hint like the evaluator's `find_file`, without erroring.
#[cfg_attr(coverage_nightly, coverage(off))]
fn resolve_hint(hint: &str, include_paths: &[PathBuf]) -> Option<PathBuf> {
    let p = Path::new(hint);
    if p.is_absolute() && p.exists() {
        return p.canonicalize().ok();
    }
    include_paths
        .iter()
        .map(|inc| inc.join(hint))
        .find(|c| c.exists())
        .and_then(|c| c.canonicalize().ok())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn escape_dep_path(p: &Path) -> String {
    p.display().to_string().replace(' ', "\\ ")
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn write_stamp(stamp: &Path, written: &[PathBuf]) -> Result<(), Error> {
    let mut content = String::new();
    for p in written {
        content.push_str(&p.display().to_string());
        content.push('\n');
    }
    std::fs::write(stamp, content)?;
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn write_depfile(
    depfile: &Path,
    stamp: Option<&Path>,
    inputs: &[PathBuf],
    includes: &[PathBuf],
) -> Result<(), Error> {
    let target = stamp.map_or_else(|| escape_dep_path(depfile), escape_dep_path);
    let mut deps: Vec<String> = inputs
        .iter()
        .chain(includes.iter())
        .map(|p| escape_dep_path(p))
        .collect();
    deps.sort();
    deps.dedup();
    std::fs::write(depfile, format!("{target}: {}\n", deps.join(" ")))?;
    Ok(())
}

fn collect_inputs(args: &Args) -> Result<Vec<PathBuf>, Error> {
    if let Some(dir) = &args.directory {
        let mut paths = Vec::new();
        for entry in WalkDir::new(dir) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(Error::Io {
                        source: std::io::Error::other(e.to_string()),
                    })
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            if has_extension(&path, &args.extensions) {
                paths.push(path);
            }
        }
        paths.sort();
        Ok(paths)
    } else {
        Ok(args.inputs.clone())
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn has_extension(path: &Path, extensions: &[String]) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => extensions.iter().any(|wanted| wanted == ext),
        None => false,
    }
}

fn apply_cli_defines(eval: &mut Evaluator, defines: &[String]) -> Result<(), EvalError> {
    for item in defines {
        let (name, value) = match item.split_once('=') {
            Some(pair) => pair,
            None => {
                return Err(EvalError::InvalidUsage(
                    None,
                    format!("define: expected NAME=VALUE, got '{item}'"),
                ))
            }
        };
        if !is_ascii_identifier(name) {
            return Err(EvalError::InvalidUsage(None, format!(
                "define: '{name}' is not a valid identifier"
            )));
        }
        eval.set_variable(name, value);
    }
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn is_ascii_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args(root: &Path) -> Args {
        Args {
            inputs: Vec::new(),
            directory: None,
            extensions: vec!["md".to_string()],
            out_dir: root.join("out"),
            no_macro: false,
            sigil: '%',
            include: vec![root.to_path_buf()],
            allow_env: false,
            env_prefix: None,
            define: Vec::new(),
            macro_only: false,
            open_delim: reweave_tangle::DEFAULT_OPEN_DELIM.to_string(),
            close_delim: reweave_tangle::DEFAULT_CLOSE_DELIM.to_string(),
            chunk_end: reweave_tangle::DEFAULT_CHUNK_END.to_string(),
            comment_markers: vec!["//".to_string(), "#".to_string()],
            formatters: Vec::new(),
            stamp: None,
            depfile: None,
            recursion_limit: 100,
        }
    }

    fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn discovers_markdown_files_with_matching_extensions_only() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.md"), "").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "").unwrap();
        let mut args = default_args(tmp.path());
        args.directory = Some(tmp.path().to_path_buf());

        assert_eq!(collect_inputs(&args).unwrap().len(), 1);
    }

    #[test]
    fn directory_discovery_reports_walk_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut args = default_args(tmp.path());
        args.directory = Some(tmp.path().join("missing"));

        assert!(matches!(collect_inputs(&args), Err(Error::Io { .. })));
    }

    #[test]
    fn run_expands_macros_and_tangles_file_chunks() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write(
            tmp.path(),
            "main.md",
            r#"
```rust
// <[@file src/main.rs]>=
fn main() {
    println!("%(message)");
}
// @
```
"#,
        );
        let mut args = default_args(tmp.path());
        args.inputs = vec![input];
        args.define = vec!["message=hello".to_string()];

        run(args).unwrap();

        let generated = std::fs::read_to_string(tmp.path().join("out/src/main.rs")).unwrap();
        assert_eq!(generated, "fn main() {\n    println!(\"hello\");\n}\n");
    }

    #[test]
    fn run_no_macro_tangles_literal_input() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\n%(literal)\n# @\n```",
        );
        let mut args = default_args(tmp.path());
        args.inputs = vec![input];
        args.no_macro = true;

        run(args).unwrap();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/out.txt")).unwrap(),
            "%(literal)\n"
        );
    }

    #[test]
    fn run_discovers_directory_inputs_in_stable_order() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "b.md",
            "```text\n# <[@file b.txt]>=\nb\n# @\n```",
        );
        write(
            tmp.path(),
            "a.md",
            "```text\n# <[@file a.txt]>=\na\n# @\n```",
        );
        write(tmp.path(), "ignored.txt", "not markdown");
        let mut args = default_args(tmp.path());
        args.directory = Some(tmp.path().to_path_buf());

        run(args).unwrap();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/a.txt")).unwrap(),
            "a\n"
        );
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/b.txt")).unwrap(),
            "b\n"
        );
    }

    #[test]
    fn run_accepts_custom_delimiters_and_comment_markers() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write(
            tmp.path(),
            "main.md",
            "```text\n; <<@file out.txt>>=\nvalue\n; @\n```",
        );
        let mut args = default_args(tmp.path());
        args.inputs = vec![input];
        args.open_delim = "<<".to_string();
        args.close_delim = ">>".to_string();
        args.comment_markers = vec![";".to_string()];

        run(args).unwrap();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/out.txt")).unwrap(),
            "value\n"
        );
    }

    #[test]
    fn run_reports_macro_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\n%(missing)\n# @\n```",
        );
        let mut args = default_args(tmp.path());
        args.inputs = vec![input];

        assert!(matches!(run(args), Err(Error::Macro { .. })));
    }

    #[test]
    fn run_reports_tangle_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\n# <[missing]>\n# @\n```",
        );
        let mut args = default_args(tmp.path());
        args.inputs = vec![input];

        assert!(matches!(run(args), Err(Error::Tangle { .. })));
    }

    #[test]
    fn apply_cli_defines_rejects_malformed_and_invalid_names() {
        let mut evaluator = Evaluator::new(EvalConfig::default());
        assert!(apply_cli_defines(&mut evaluator, &["missing_equals".to_string()]).is_err());
        assert!(apply_cli_defines(&mut evaluator, &["1bad=value".to_string()]).is_err());
    }

    #[test]
    fn has_extension_handles_missing_extension() {
        assert!(!has_extension(Path::new("README"), &["md".to_string()]));
        assert!(has_extension(Path::new("README.md"), &["md".to_string()]));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn args_for(root: &Path, inputs: Vec<PathBuf>) -> Args {
        Args {
            inputs,
            directory: None,
            extensions: vec!["md".to_string()],
            out_dir: root.join("out"),
            no_macro: false,
            sigil: '%',
            include: vec![root.to_path_buf()],
            allow_env: false,
            env_prefix: None,
            define: Vec::new(),
            macro_only: false,
            open_delim: reweave_tangle::DEFAULT_OPEN_DELIM.to_string(),
            close_delim: reweave_tangle::DEFAULT_CLOSE_DELIM.to_string(),
            chunk_end: reweave_tangle::DEFAULT_CHUNK_END.to_string(),
            comment_markers: vec!["//".to_string(), "#".to_string()],
            formatters: Vec::new(),
            stamp: None,
            depfile: None,
            recursion_limit: 100,
        }
    }

    fn write_file(root: &Path, rel: &str, text: &str) -> PathBuf {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn fragment_included_by_main_is_not_tangled_standalone() {
        let tmp = tempfile::tempdir().unwrap();
        // The fragment sorts before the main document and uses a macro the
        // main document defines — it can only be expanded via the include.
        let fragment = write_file(
            tmp.path(),
            "a_fragment.md",
            "```text\n# <[body]>=\n%greet()\n# @\n```",
        );
        let main_doc = write_file(
            tmp.path(),
            "z_main.md",
            "%def(greet, hello)\n%include(a_fragment.md)\n```text\n# <[@file out.txt]>=\n# <[body]>\n# @\n```",
        );
        let mut args = args_for(tmp.path(), vec![fragment, main_doc]);
        args.stamp = Some(tmp.path().join("gen.stamp"));
        args.depfile = Some(tmp.path().join("gen.d"));

        run(args).unwrap();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/out.txt")).unwrap(),
            "hello\n"
        );
        // Stamp lists the one written output.
        let stamp = std::fs::read_to_string(tmp.path().join("gen.stamp")).unwrap();
        assert!(stamp.contains("out.txt"));
        // Depfile covers both the main document and the fragment.
        let dep = std::fs::read_to_string(tmp.path().join("gen.d")).unwrap();
        assert!(dep.contains("z_main.md"));
        assert!(dep.contains("a_fragment.md"));
    }

    #[test]
    fn macro_computed_include_falls_back_to_deferral_pass() {
        let tmp = tempfile::tempdir().unwrap();
        // The include argument is macro-computed, so the static hint scan
        // misses it: the fragment is only claimed after the main document
        // expands — the deferral pass retries and skips it.
        let fragment = write_file(
            tmp.path(),
            "a_frag.md",
            "```text\n# <[body]>=\n%greet()\n# @\n```",
        );
        let main_doc = write_file(
            tmp.path(),
            "z_main.md",
            "%def(greet, hi)\n%set(f, a_frag.md)\n%include(%(f))\n```text\n# <[@file out.txt]>=\n# <[body]>\n# @\n```",
        );
        let args = args_for(tmp.path(), vec![fragment, main_doc]);

        run(args).unwrap();

        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/out.txt")).unwrap(),
            "hi\n"
        );
    }

    #[test]
    fn second_run_writes_nothing_and_stamp_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write_file(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\nhello\n# @\n```",
        );
        let make_args = || {
            let mut a = args_for(tmp.path(), vec![input.clone()]);
            a.stamp = Some(tmp.path().join("gen.stamp"));
            a
        };

        run(make_args()).unwrap();
        let first = std::fs::read_to_string(tmp.path().join("gen.stamp")).unwrap();
        assert!(first.contains("out.txt"));

        run(make_args()).unwrap();
        let second = std::fs::read_to_string(tmp.path().join("gen.stamp")).unwrap();
        assert_eq!(second, "");
    }

    #[test]
    fn formatter_runs_before_change_detection() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write_file(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\nhello\n# @\n```",
        );
        let fmt = write_file(
            tmp.path(),
            "fmt.sh",
            "#!/bin/sh\nprintf 'formatted\\n' >> \"$1\"\n",
        );
        // Make the formatter executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fmt, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let make_args = || {
            let mut a = args_for(tmp.path(), vec![input.clone()]);
            a.formatters = vec![fmt.to_string_lossy().into_owned()];
            a.stamp = Some(tmp.path().join("gen.stamp"));
            a
        };

        run(make_args()).unwrap();
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out/out.txt")).unwrap(),
            "hello\nformatted\n"
        );

        // The formatted content is what change detection compares against,
        // so the second run is a no-op rather than a churn loop.
        run(make_args()).unwrap();
        let second = std::fs::read_to_string(tmp.path().join("gen.stamp")).unwrap();
        assert_eq!(second, "");
    }

    #[test]
    fn formatter_failure_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let input = write_file(
            tmp.path(),
            "main.md",
            "```text\n# <[@file out.txt]>=\nhello\n# @\n```",
        );
        // Runs but exits non-zero, exercising the FormatterFailed path.
        let fmt = write_file(tmp.path(), "fail.sh", "#!/bin/sh\nexit 1\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fmt, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut args = args_for(tmp.path(), vec![input]);
        args.formatters = vec![fmt.to_string_lossy().into_owned()];

        match run(args).unwrap_err() {
            Error::Tangle { source } => assert!(
                source.to_string().contains("fail.sh"),
                "expected FormatterFailed naming the script, got: {source}"
            ),
            other => panic!("expected Tangle error, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;

    #[test]
    fn include_hints_finds_literal_includes_and_imports() {
        let text = "%include(src/a.md)\ntext\n%import( b.md )\n%include(%(dynamic))\n%include(also-macro(1))";
        let hints = include_hints(text);
        assert_eq!(hints, vec!["src/a.md", "b.md"]);
    }

    #[test]
    fn resolve_hint_resolves_absolute_and_include_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let abs = tmp.path().join("abs.md");
        std::fs::write(&abs, "x").unwrap();
        assert_eq!(
            resolve_hint(abs.to_str().unwrap(), &[]),
            Some(abs.clone())
        );

        let inc = tmp.path().join("inc");
        std::fs::create_dir(&inc).unwrap();
        std::fs::write(inc.join("frag.md"), "x").unwrap();
        assert_eq!(
            resolve_hint("frag.md", &[inc.clone()]),
            Some(inc.join("frag.md"))
        );

        assert_eq!(resolve_hint("missing.md", &[inc]), None);
    }

    #[test]
    fn schedule_emits_main_documents_before_their_fragments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let frag = root.join("frag.md");
        let main = root.join("main.md");
        let plain = root.join("plain.md");
        std::fs::write(&frag, "").unwrap();
        std::fs::write(&main, "%include(frag.md)").unwrap();
        std::fs::write(&plain, "").unwrap();

        let inputs = vec![frag.clone(), main.clone(), plain.clone()];
        let canonical: std::collections::HashMap<PathBuf, PathBuf> = inputs
            .iter()
            .map(|p| (p.canonicalize().unwrap(), p.clone()))
            .collect();
        let order = schedule(&inputs, &[root.to_path_buf()], &canonical);

        let pos = |p: &PathBuf| order.iter().position(|x| x == p).unwrap();
        assert!(pos(&main) < pos(&frag));
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn schedule_skips_hints_that_do_not_resolve_to_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let main = root.join("main.md");
        let outside = root.join("outside.md");
        let plain = root.join("plain.md");
        // One hint points nowhere, one resolves outside the input set.
        std::fs::write(&main, "%include(missing.md)\n%include(outside.md)").unwrap();
        std::fs::write(&outside, "").unwrap();
        std::fs::write(&plain, "").unwrap();

        let inputs = vec![main.clone(), plain.clone()];
        let canonical: std::collections::HashMap<PathBuf, PathBuf> = inputs
            .iter()
            .map(|p| (p.canonicalize().unwrap(), p.clone()))
            .collect();
        let order = schedule(&inputs, &[root.to_path_buf()], &canonical);

        // No hinted targets resolve into the input set, so input order is kept.
        assert_eq!(order, inputs);
    }

    #[test]
    fn schedule_tolerates_unreadable_inputs() {
        let missing = PathBuf::from("/definitely/missing.md");
        let inputs = vec![missing.clone()];
        let canonical = std::collections::HashMap::new();
        let order = schedule(&inputs, &[], &canonical);
        assert_eq!(order, vec![missing]);
    }

    #[test]
    fn depfile_without_stamp_targets_itself_and_escapes_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let dep = tmp.path().join("gen.d");
        let input = tmp.path().join("my file.md");
        std::fs::write(&input, "").unwrap();

        write_depfile(&dep, None, &[input.clone()], &[]).unwrap();
        let content = std::fs::read_to_string(&dep).unwrap();
        assert_eq!(
            content,
            format!("{}: {}\n", dep.display(), input.display().to_string().replace(' ', "\\ "))
        );
    }

    #[test]
    fn write_stamp_lists_written_files() {
        let tmp = tempfile::tempdir().unwrap();
        let stamp = tmp.path().join("gen.stamp");
        let written = vec![tmp.path().join("a.txt"), tmp.path().join("b.txt")];
        write_stamp(&stamp, &written).unwrap();
        assert_eq!(
            std::fs::read_to_string(&stamp).unwrap(),
            format!("{}/a.txt\n{}/b.txt\n", tmp.path().display(), tmp.path().display())
        );
    }

    #[test]
    fn genuine_expansion_failure_is_reported_after_deferral() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("bad.md");
        std::fs::write(&input, "%(undefined_variable)").unwrap();
        let mut evaluator = Evaluator::new(EvalConfig::default());

        let err = expand_inputs(&[input], &mut evaluator, &[tmp.path().to_path_buf()], false)
            .unwrap_err();
        assert!(matches!(err, Error::Macro { .. }));
    }

    #[test]
    fn no_macro_expands_all_inputs_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("plain.md");
        std::fs::write(&input, "%(literal)").unwrap();
        let mut evaluator = Evaluator::new(EvalConfig::default());

        let expanded = expand_inputs(&[input], &mut evaluator, &[], true).unwrap();
        assert_eq!(expanded.mains.len(), 1);
        assert_eq!(expanded.mains[0].1, "%(literal)");
        assert!(expanded.includes.is_empty());
    }
}
