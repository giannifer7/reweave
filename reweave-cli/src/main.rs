use std::path::{Path, PathBuf};

use clap::Parser;
use miette::Diagnostic;
use reweave_macro::evaluator::{EvalConfig, EvalError, Evaluator};
use reweave_macro::macro_api::process_string;
use reweave_tangle::{Tangle, TangleConfig, TangleError};
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

    /// Maximum recursion depth for macro and chunk expansion.
    #[arg(long = "recursion-limit", default_value_t = reweave_core::MAX_RECURSION_DEPTH)]
    recursion_limit: usize,
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    run(args).map_err(miette::Report::new)
}

fn run(args: Args) -> Result<(), Error> {
    let inputs = collect_inputs(&args)?;
    let mut tangle = Tangle::new(TangleConfig {
        open_delim: args.open_delim,
        close_delim: args.close_delim,
        chunk_end: args.chunk_end,
        comment_markers: args.comment_markers,
        strict_undefined: true,
        recursion_limit: args.recursion_limit,
    });

    let mut evaluator = Evaluator::new(EvalConfig {
        sigil: args.sigil,
        include_paths: args.include,
        allow_env: args.allow_env,
        env_prefix: args.env_prefix,
        recursion_limit: args.recursion_limit,
    });
    apply_cli_defines(&mut evaluator, &args.define)?;

    for input in inputs {
        let text = std::fs::read_to_string(&input)?;
        let expanded = if args.no_macro {
            text
        } else {
            let bytes = process_string(&text, Some(&input), &mut evaluator)?;
            String::from_utf8_lossy(&bytes).into_owned()
        };
        tangle.read(&expanded, &input.to_string_lossy());
    }

    tangle.write_files(&args.out_dir)?;
    Ok(())
}

fn collect_inputs(args: &Args) -> Result<Vec<PathBuf>, Error> {
    if let Some(dir) = &args.directory {
        let mut paths = Vec::new();
        for entry in WalkDir::new(dir) {
            let entry = entry.map_err(|e| Error::Io {
                source: std::io::Error::other(e.to_string()),
            })?;
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

fn has_extension(path: &Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.iter().any(|wanted| wanted == ext))
}

fn apply_cli_defines(eval: &mut Evaluator, defines: &[String]) -> Result<(), EvalError> {
    for item in defines {
        let (name, value) = item.split_once('=').ok_or_else(|| {
            EvalError::InvalidUsage(format!("define: expected NAME=VALUE, got '{item}'"))
        })?;
        if !is_ascii_identifier(name) {
            return Err(EvalError::InvalidUsage(format!(
                "define: '{name}' is not a valid identifier"
            )));
        }
        eval.set_variable(name, value);
    }
    Ok(())
}

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
            open_delim: reweave_tangle::DEFAULT_OPEN_DELIM.to_string(),
            close_delim: reweave_tangle::DEFAULT_CLOSE_DELIM.to_string(),
            chunk_end: reweave_tangle::DEFAULT_CHUNK_END.to_string(),
            comment_markers: vec!["//".to_string(), "#".to_string()],
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
