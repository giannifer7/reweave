use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use thiserror::Error;

pub const DEFAULT_OPEN_DELIM: &str = "<[";
pub const DEFAULT_CLOSE_DELIM: &str = "]>";
pub const DEFAULT_CHUNK_END: &str = "@";

#[derive(Debug, Clone, Error)]
pub enum TangleError {
    #[error("{file_name} line {}: maximum recursion depth exceeded while expanding chunk '{chunk}'", line + 1)]
    RecursionLimit {
        chunk: String,
        file_name: String,
        line: usize,
    },
    #[error("{file_name} line {}: recursive reference detected in chunk '{chunk}' (cycle: {})", line + 1, cycle.join(" -> "))]
    RecursiveReference {
        chunk: String,
        cycle: Vec<String>,
        file_name: String,
        line: usize,
    },
    #[error("{file_name} line {}: referenced chunk '{chunk}' is undefined", line + 1)]
    UndefinedChunk {
        chunk: String,
        file_name: String,
        line: usize,
    },
    #[error("{file_name} line {}: file chunk '{file_chunk}' is already defined (use @replace to redefine)", line + 1)]
    FileChunkRedefinition {
        file_chunk: String,
        file_name: String,
        line: usize,
    },
    #[error("unsafe @file path '{path}'")]
    UnsafePath { path: String },
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<io::Error> for TangleError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkDefinitionMatch {
    pub indent_len: usize,
    pub base_name: String,
    pub is_replace: bool,
    pub is_file: bool,
}

#[derive(Debug, Clone)]
struct ChunkReferenceMatch {
    add_indent: String,
    modifier: String,
    referenced_chunk: String,
}

#[derive(Debug, Clone)]
pub struct NowebSyntax {
    open_re: Regex,
    slot_re: Regex,
    close_re: Regex,
    open_bytes: Box<[u8]>,
    close_bytes: Box<[u8]>,
}

impl NowebSyntax {
    pub fn new(
        open_delim: &str,
        close_delim: &str,
        chunk_end: &str,
        comment_markers: &[String],
    ) -> Self {
        let od = regex::escape(open_delim);
        let cd = regex::escape(close_delim);
        let comments = comment_markers
            .iter()
            .map(|m| regex::escape(m))
            .collect::<Vec<_>>()
            .join("|");

        let open_pattern = format!(
            r"^(?P<indent>\s*)(?:{})?[ \t]*{}(?P<replace>@replace[ \t]+)?(?P<file>@file[ \t]+)?(?P<name>.+?){}=[ \t]*$",
            comments, od, cd
        );
        let slot_pattern = format!(
            r"^(\s*)(?:{})?\s*{}((?:(?:@file|@reversed|@compact|@tight)\s+)*)?(.+?){}\s*$",
            comments, od, cd
        );
        let close_pattern = format!(r"^(?:{})?[ \t]*{}\s*$", comments, regex::escape(chunk_end));

        Self {
            open_re: Regex::new(&open_pattern).expect("valid chunk-open regex"),
            slot_re: Regex::new(&slot_pattern).expect("valid chunk-reference regex"),
            close_re: Regex::new(&close_pattern).expect("valid chunk-close regex"),
            open_bytes: open_delim.as_bytes().into(),
            close_bytes: chunk_end.as_bytes().into(),
        }
    }

    pub fn parse_definition_line(&self, line: &str) -> Option<ChunkDefinitionMatch> {
        memchr::memmem::find(line.as_bytes(), &self.open_bytes)?;
        let caps = self.open_re.captures(line)?;
        Some(ChunkDefinitionMatch {
            indent_len: caps.name("indent").map_or("", |m| m.as_str()).len(),
            base_name: caps.name("name").map_or("", |m| m.as_str()).to_string(),
            is_replace: caps.name("replace").is_some(),
            is_file: caps.name("file").is_some(),
        })
    }

    pub fn is_close_line(&self, line: &str) -> bool {
        memchr::memmem::find(line.as_bytes(), &self.close_bytes).is_some()
            && self.close_re.is_match(line)
    }

    fn parse_reference_line(&self, line: &str) -> Option<ChunkReferenceMatch> {
        memchr::memmem::find(line.as_bytes(), &self.open_bytes)?;
        let caps = self.slot_re.captures(line)?;
        Some(ChunkReferenceMatch {
            add_indent: caps.get(1).map_or("", |m| m.as_str()).to_string(),
            modifier: caps.get(2).map_or("", |m| m.as_str()).to_string(),
            referenced_chunk: caps.get(3).map_or("", |m| m.as_str()).to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct ChunkDef {
    content: Vec<String>,
    base_indent: usize,
    file_idx: usize,
    line: usize,
}

#[derive(Debug, Clone)]
struct NamedChunk {
    definitions: Vec<ChunkDef>,
}

impl NamedChunk {
    fn new() -> Self {
        Self {
            definitions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TangleConfig {
    pub open_delim: String,
    pub close_delim: String,
    pub chunk_end: String,
    pub comment_markers: Vec<String>,
    pub strict_undefined: bool,
    pub recursion_limit: usize,
}

impl Default for TangleConfig {
    fn default() -> Self {
        Self {
            open_delim: DEFAULT_OPEN_DELIM.to_string(),
            close_delim: DEFAULT_CLOSE_DELIM.to_string(),
            chunk_end: DEFAULT_CHUNK_END.to_string(),
            comment_markers: vec!["//".to_string(), "#".to_string()],
            strict_undefined: true,
            recursion_limit: 100,
        }
    }
}

#[derive(Debug)]
pub struct Tangle {
    chunks: HashMap<String, NamedChunk>,
    file_chunks: Vec<String>,
    syntax: NowebSyntax,
    file_names: Vec<String>,
    strict_undefined: bool,
    recursion_limit: usize,
    parse_errors: Vec<TangleError>,
}

impl Tangle {
    pub fn new(config: TangleConfig) -> Self {
        Self {
            chunks: HashMap::new(),
            file_chunks: Vec::new(),
            syntax: NowebSyntax::new(
                &config.open_delim,
                &config.close_delim,
                &config.chunk_end,
                &config.comment_markers,
            ),
            file_names: Vec::new(),
            strict_undefined: config.strict_undefined,
            recursion_limit: config.recursion_limit,
            parse_errors: Vec::new(),
        }
    }

    pub fn read_file(&mut self, path: &Path) -> Result<(), TangleError> {
        let name = path.to_string_lossy().to_string();
        let idx = self.add_file_name(&name);
        let text = if path == Path::new("-") {
            let mut buf = String::new();
            io::stdin().lock().read_to_string(&mut buf)?;
            buf
        } else {
            fs::read_to_string(path)?
        };
        self.read_with_idx(&text, idx);
        Ok(())
    }

    pub fn read(&mut self, text: &str, file_name: &str) {
        let idx = self.add_file_name(file_name);
        self.read_with_idx(text, idx);
    }

    pub fn file_chunks(&self) -> &[String] {
        &self.file_chunks
    }

    pub fn has_chunk(&self, name: &str) -> bool {
        self.chunks.contains_key(name)
    }

    pub fn expand(&self, chunk_name: &str) -> Result<Vec<String>, TangleError> {
        if self.strict_undefined
            && let Some(err) = self.parse_errors.first()
        {
            return Err(err.clone());
        }
        let mut state = ExpandState::new();
        self.expand_inner(chunk_name, "", &mut state, 0, 0, RefOptions::default())
    }

    pub fn write_files(&self, out_dir: &Path) -> Result<Vec<PathBuf>, TangleError> {
        let mut written = Vec::new();
        for name in self.file_chunks() {
            let rel = name.strip_prefix("@file ").unwrap_or(name).trim();
            path_is_safe(rel)?;
            let out_path = out_dir.join(rel);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(&out_path)?;
            for line in self.expand(name)? {
                file.write_all(line.as_bytes())?;
            }
            written.push(out_path);
        }
        Ok(written)
    }

    fn add_file_name(&mut self, fname: &str) -> usize {
        let idx = self.file_names.len();
        self.file_names.push(fname.to_string());
        idx
    }

    fn read_with_idx(&mut self, text: &str, file_idx: usize) {
        let mut current_chunk: Option<(String, usize)> = None;

        for (line_no, line) in text.lines().enumerate() {
            if let Some(def_match) = self.syntax.parse_definition_line(line) {
                let full_name = if def_match.is_file {
                    format!("@file {}", def_match.base_name)
                } else {
                    def_match.base_name
                };

                if let Err(err) = self.validate_chunk_name(&full_name, def_match.is_file) {
                    self.parse_errors.push(err);
                    current_chunk = None;
                    continue;
                }

                if full_name.starts_with("@file ") {
                    if self.chunks.contains_key(&full_name) && !def_match.is_replace {
                        self.parse_errors.push(TangleError::FileChunkRedefinition {
                            file_chunk: full_name,
                            file_name: self.file_name(file_idx),
                            line: line_no,
                        });
                        current_chunk = None;
                        continue;
                    }
                    if def_match.is_replace {
                        self.chunks.remove(&full_name);
                    }
                } else if def_match.is_replace {
                    self.chunks.remove(&full_name);
                }

                let chunk = self
                    .chunks
                    .entry(full_name.clone())
                    .or_insert_with(NamedChunk::new);
                let def_idx = chunk.definitions.len();
                chunk.definitions.push(ChunkDef {
                    content: Vec::new(),
                    base_indent: def_match.indent_len,
                    file_idx,
                    line: line_no,
                });
                current_chunk = Some((full_name.clone(), def_idx));
                if full_name.starts_with("@file ") && !self.file_chunks.contains(&full_name) {
                    self.file_chunks.push(full_name);
                }
                continue;
            }

            if self.syntax.is_close_line(line) {
                current_chunk = None;
                continue;
            }

            if let Some((ref name, idx)) = current_chunk
                && let Some(chunk) = self.chunks.get_mut(name)
            {
                chunk.definitions[idx].content.push(format!("{line}\n"));
            }
        }
    }

    fn validate_chunk_name(&self, chunk_name: &str, is_file: bool) -> Result<(), TangleError> {
        if is_file {
            let path = chunk_name.strip_prefix("@file ").unwrap_or(chunk_name);
            path_is_safe(path)
        } else if chunk_name.is_empty() {
            Err(TangleError::UnsafePath {
                path: chunk_name.to_string(),
            })
        } else {
            Ok(())
        }
    }

    fn expand_inner(
        &self,
        chunk_name: &str,
        target_indent: &str,
        state: &mut ExpandState,
        ref_file_idx: usize,
        ref_line: usize,
        options: RefOptions,
    ) -> Result<Vec<String>, TangleError> {
        if state.stack.len() > self.recursion_limit {
            return Err(TangleError::RecursionLimit {
                chunk: chunk_name.to_string(),
                file_name: self.file_name(ref_file_idx),
                line: ref_line,
            });
        }
        if state.seen.contains(chunk_name) {
            let mut cycle = state.stack.clone();
            cycle.push(chunk_name.to_string());
            return Err(TangleError::RecursiveReference {
                chunk: chunk_name.to_string(),
                cycle,
                file_name: self.file_name(ref_file_idx),
                line: ref_line,
            });
        }

        let Some(chunk) = self.chunks.get(chunk_name) else {
            if self.strict_undefined {
                return Err(TangleError::UndefinedChunk {
                    chunk: chunk_name.to_string(),
                    file_name: self.file_name(ref_file_idx),
                    line: ref_line,
                });
            }
            return Ok(Vec::new());
        };

        state.seen.insert(chunk_name.to_string());
        state.stack.push(chunk_name.to_string());

        let indices: Vec<usize> = if options.reversed {
            (0..chunk.definitions.len()).rev().collect()
        } else {
            (0..chunk.definitions.len()).collect()
        };

        let mut result = Vec::new();
        for def_idx in indices {
            let def = &chunk.definitions[def_idx];
            let mut def_result = Vec::new();
            for (line_count, line) in def.content.iter().enumerate() {
                if let Some(slot) = self.syntax.parse_reference_line(line) {
                    let child_options = RefOptions {
                        reversed: slot.modifier.contains("@reversed"),
                        compact: slot.modifier.contains("@compact"),
                        tight: slot.modifier.contains("@tight"),
                    };
                    let relative_indent = if slot.add_indent.len() > def.base_indent {
                        &slot.add_indent[def.base_indent..]
                    } else {
                        ""
                    };
                    let new_indent = format!("{target_indent}{relative_indent}");
                    let child = self.expand_inner(
                        slot.referenced_chunk.trim(),
                        &new_indent,
                        state,
                        def.file_idx,
                        def.line + line_count,
                        child_options,
                    )?;
                    def_result.extend(apply_ref_space_options(child, child_options));
                } else {
                    let line_indent = if line.len() > def.base_indent {
                        &line[def.base_indent..]
                    } else {
                        line
                    };
                    def_result.push(format!("{target_indent}{line_indent}"));
                }
            }
            result.extend(apply_ref_space_options(def_result, options));
        }

        state.stack.pop();
        state.seen.remove(chunk_name);
        Ok(result)
    }

    fn file_name(&self, idx: usize) -> String {
        self.file_names.get(idx).cloned().unwrap_or_default()
    }
}

#[derive(Debug)]
struct ExpandState {
    seen: HashSet<String>,
    stack: Vec<String>,
}

impl ExpandState {
    fn new() -> Self {
        Self {
            seen: HashSet::new(),
            stack: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RefOptions {
    reversed: bool,
    compact: bool,
    tight: bool,
}

fn apply_ref_space_options(mut lines: Vec<String>, options: RefOptions) -> Vec<String> {
    if options.compact || options.tight {
        lines = trim_blank_edge_lines(lines);
    }
    if options.tight {
        lines.retain(|line| !line.trim().is_empty());
    }
    lines
}

fn trim_blank_edge_lines(lines: Vec<String>) -> Vec<String> {
    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    lines
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn path_is_safe(path: &str) -> Result<(), TangleError> {
    let p = Path::new(path);
    if p.is_absolute() {
        return Err(TangleError::UnsafePath {
            path: path.to_string(),
        });
    }
    for component in p.components() {
        if matches!(component, Component::ParentDir | Component::Prefix(_)) {
            return Err(TangleError::UnsafePath {
                path: path.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(src: &str) -> Tangle {
        let mut t = Tangle::new(TangleConfig::default());
        t.read(src, "test.md");
        t
    }

    fn read_with_config(src: &str, config: TangleConfig) -> Tangle {
        let mut t = Tangle::new(config);
        t.read(src, "test.md");
        t
    }

    #[test]
    fn expands_file_chunks() {
        let t = read(
            r#"
```rust
// <[@file src/main.rs]>=
fn main() {
    // <[body]>
}
// @

// <[body]>=
println!("hi");
// @
```
"#,
        );
        assert_eq!(
            t.expand("@file src/main.rs").unwrap().join(""),
            "fn main() {\n    println!(\"hi\");\n}\n"
        );
    }

    #[test]
    fn rejects_undefined_chunks_by_default() {
        let t = read("```text\n# <[@file out.txt]>=\n# <[missing]>\n# @\n```");
        assert!(matches!(
            t.expand("@file out.txt").unwrap_err(),
            TangleError::UndefinedChunk { .. }
        ));
    }

    #[test]
    fn supports_reversed_modifier() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[@reversed rows]>\n# @\n# <[rows]>=\na\n# @\n# <[rows]>=\nb\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "b\na\n");
    }

    #[test]
    fn supports_compact_and_tight_modifiers() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[@tight body]>\n# @\n# <[body]>=\n\nx\n\n\ny\n\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\ny\n");
    }

    #[test]
    fn compact_trims_only_edge_blank_lines() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[@compact body]>\n# @\n# <[body]>=\n\nx\n\ny\n\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\n\ny\n");
    }

    #[test]
    fn replace_redefines_file_chunk() {
        let t = read(
            "```text\n# <[@file out.txt]>=\nold\n# @\n# <[@replace @file out.txt]>=\nnew\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "new\n");
    }

    #[test]
    fn replace_redefines_named_chunk() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[body]>\n# @\n# <[body]>=\nold\n# @\n# <[@replace body]>=\nnew\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "new\n");
    }

    #[test]
    fn named_chunks_accumulate_in_definition_order() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[rows]>\n# @\n# <[rows]>=\na\n# @\n# <[rows]>=\nb\n# @\n```",
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "a\nb\n");
    }

    #[test]
    fn duplicate_file_chunk_is_an_error() {
        let t =
            read("```text\n# <[@file out.txt]>=\none\n# @\n# <[@file out.txt]>=\ntwo\n# @\n```");
        assert!(matches!(
            t.expand("@file out.txt").unwrap_err(),
            TangleError::FileChunkRedefinition { .. }
        ));
    }

    #[test]
    fn rejects_path_traversal_outputs() {
        let t = read("```text\n# <[@file ../x.txt]>=\nx\n# @\n```");
        assert!(matches!(
            t.expand("@file ../x.txt").unwrap_err(),
            TangleError::UnsafePath { .. } | TangleError::UndefinedChunk { .. }
        ));
    }

    #[test]
    fn rejects_absolute_file_chunk_paths() {
        let t = read("```text\n# <[@file /tmp/out.txt]>=\nx\n# @\n```");
        assert!(matches!(
            t.expand("@file /tmp/out.txt").unwrap_err(),
            TangleError::UnsafePath { .. } | TangleError::UndefinedChunk { .. }
        ));
    }

    #[test]
    fn non_strict_undefined_chunks_expand_to_empty() {
        let t = read_with_config(
            "```text\n# <[@file out.txt]>=\nfirst\n# <[missing]>\nlast\n# @\n```",
            TangleConfig {
                strict_undefined: false,
                ..TangleConfig::default()
            },
        );
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "first\nlast\n");
    }

    #[test]
    fn detects_recursive_chunk_references() {
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[a]>\n# @\n# <[a]>=\n# <[b]>\n# @\n# <[b]>=\n# <[a]>\n# @\n```",
        );
        let err = t.expand("@file out.txt").unwrap_err();
        assert!(matches!(err, TangleError::RecursiveReference { .. }));
        assert!(err.to_string().contains("a -> b -> a"));
    }

    #[test]
    fn enforces_recursion_limit() {
        let t = read_with_config(
            "```text\n# <[@file out.txt]>=\n# <[a]>\n# @\n# <[a]>=\n# <[b]>\n# @\n# <[b]>=\nvalue\n# @\n```",
            TangleConfig {
                recursion_limit: 1,
                ..TangleConfig::default()
            },
        );
        assert!(matches!(
            t.expand("@file out.txt").unwrap_err(),
            TangleError::RecursionLimit { .. }
        ));
    }

    #[test]
    fn read_file_reads_chunk_source_from_disk() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("input.md");
        fs::write(&input, "```text\n# <[@file out.txt]>=\nhello\n# @\n```").unwrap();
        let mut t = Tangle::new(TangleConfig::default());

        t.read_file(&input).unwrap();

        assert!(t.has_chunk("@file out.txt"));
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "hello\n");
    }

    #[test]
    fn read_file_reports_io_errors() {
        let mut t = Tangle::new(TangleConfig::default());
        let err = t.read_file(Path::new("missing-file.md")).unwrap_err();
        assert!(matches!(err, TangleError::Io(_)));
    }

    #[test]
    fn writes_outputs_directly() {
        let temp = tempfile::tempdir().unwrap();
        let t = read("```text\n# <[@file out.txt]>=\nhello\n# @\n```");
        let written = t.write_files(temp.path()).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "hello\n"
        );
    }

    #[test]
    fn writes_nested_outputs_directly() {
        let temp = tempfile::tempdir().unwrap();
        let t = read("```text\n# <[@file nested/out.txt]>=\nhello\n# @\n```");
        let written = t.write_files(temp.path()).unwrap();

        assert_eq!(written, vec![temp.path().join("nested/out.txt")]);
        assert_eq!(
            fs::read_to_string(temp.path().join("nested/out.txt")).unwrap(),
            "hello\n"
        );
    }

    #[test]
    fn write_files_reports_directory_creation_errors() {
        let temp = tempfile::tempdir().unwrap();
        let blocked = temp.path().join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        let t = read("```text\n# <[@file nested/out.txt]>=\nx\n# @\n```");

        let err = t.write_files(&blocked).unwrap_err();

        assert!(matches!(err, TangleError::Io(_)));
    }

    #[test]
    fn reports_parse_error_before_writing_duplicate_file_chunk() {
        let temp = tempfile::tempdir().unwrap();
        let t =
            read("```text\n# <[@file out.txt]>=\none\n# @\n# <[@file out.txt]>=\ntwo\n# @\n```");

        assert!(matches!(
            t.write_files(temp.path()).unwrap_err(),
            TangleError::FileChunkRedefinition { .. }
        ));
    }

    #[test]
    fn ignores_text_outside_chunks_and_plain_close_lines() {
        let t = read("outside\n# @\n```text\n# <[@file out.txt]>=\ninside\n# @\nafter\n```");

        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "inside\n");
    }

    #[test]
    fn handles_reference_indent_shorter_than_definition_indent() {
        let t = read(
            "```text\n  # <[@file out.txt]>=\n# <[body]>\n# @\n# <[body]>=\nx\n# @\n```",
        );

        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\n");
    }

    #[test]
    fn keeps_content_lines_shorter_than_definition_indent() {
        let t = read("```text\n    # <[@file out.txt]>=\nx\n# @\n```");
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\n");
    }
}
