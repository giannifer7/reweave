#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
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
    #[error(
        "{file_name} line {}: chunk '{chunk}' is defined but never referenced from any @file chunk{}",
        line + 1,
        if group.is_empty() {
            String::new()
        } else {
            format!(" (dead together with: {})", group.join(", "))
        }
    )]
    UnreferencedChunk {
        chunk: String,
        file_name: String,
        line: usize,
        /// The rest of this chunk's dead cluster (chunks reachable only
        /// through `chunk`, or through which `chunk` is reachable): other
        /// chunks that silently disappear for the same underlying reason,
        /// reported together rather than as separate errors.
        group: Vec<String>,
    },
    #[error("{file_name} line {}: file chunk '{file_chunk}' is already defined (use @replace to redefine)", line + 1)]
    FileChunkRedefinition {
        file_chunk: String,
        file_name: String,
        line: usize,
    },
    #[error("unsafe @file path '{path}'")]
    UnsafePath { path: String },
    #[error("formatter '{command}' failed with status {status} on '{path}'")]
    FormatterFailed {
        command: String,
        path: String,
        status: String,
    },
    #[error("formatter command '{command}' could not be split into words (check quoting)")]
    FormatterCommandInvalid { command: String },
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
    /// External commands applied to every output before writing, in order.
    /// Each is invoked as `CMD <file>` and may rewrite the file in place
    /// (e.g. `nph`). Runs before the content-aware comparison, so a
    /// formatter does not defeat change detection.
    pub formatters: Vec<String>,
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
            formatters: Vec::new(),
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
    formatters: Vec<String>,
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
            formatters: config.formatters,
            parse_errors: Vec::new(),
        }
    }

    pub fn read_file(&mut self, path: &Path) -> Result<(), TangleError> {
        let name = path.to_string_lossy().to_string();
        let idx = self.add_file_name(&name);
        let text = read_path(path)?;
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

    /// Write every `@file` chunk under `out_dir`.
    ///
    /// Writes are content-aware: files whose current content already matches
    /// the expanded output are left untouched (and their mtimes preserved),
    /// so downstream build tools are not needlessly re-triggered.
    /// Returns the paths of the files that were actually written.
    ///
    /// With formatters configured, a small cache under `out_dir/.reweave/`
    /// records the raw (pre-format) expansion and a hash of the written
    /// output. On the next run, an output is skipped *without running the
    /// formatter* when both the raw expansion is unchanged and the output
    /// file is untouched since the last write. The cache is an optimization
    /// only: it is never consulted for correctness, and deleting it costs
    /// one slower run.
    ///
    /// A single tangle can write outputs anywhere `@file` chunks name, a set
    /// not known until every chunk has been scanned — so each configured
    /// formatter runs *once per `write_files` call*, across every output that
    /// actually needs (re)formatting, rather than once per file: a JVM-booting
    /// formatter like `clojure -M:cljfmt fix` costs the same whether it is
    /// handed one path or a hundred, and a hundred separate process spawns for
    /// a hundred tangled files would not scale.
    pub fn write_files(&self, out_dir: &Path) -> Result<Vec<PathBuf>, TangleError> {
        // A named chunk nobody ever references is as much a structural
        // mistake as a reference to a chunk nobody ever defines (the
        // existing `UndefinedChunk` check, the other direction): it drops
        // out of the generated output with nothing in the run's own output
        // to say why. Checked once here, across every chunk this run
        // parsed, rather than per `@file` chunk below.
        if let Some(err) = self.unreferenced_chunk_errors().into_iter().next() {
            return Err(err);
        }

        let mut written = Vec::new();
        let mut to_format: Vec<(PathBuf, String, Vec<u8>)> = Vec::new();

        for name in self.file_chunks() {
            let rel = name.strip_prefix("@file ").unwrap_or(name).trim().to_string();
            path_is_safe(&rel)?;
            let out_path = out_dir.join(&rel);
            let mut raw = Vec::new();
            for line in self.expand(name)? {
                raw.extend_from_slice(line.as_bytes());
            }

            // Cheap pre-check: identical to the raw expansion (covers the
            // no-formatter case and identity formatters).
            if fs::read(&out_path).is_ok_and(|existing| existing == raw) {
                continue;
            }

            if self.formatters.is_empty() {
                ensure_parent_dir(&out_path)?;
                fs::write(&out_path, &raw)?;
                written.push(out_path);
                continue;
            }

            // Verified cache hit: raw expansion unchanged AND output file
            // untouched — this output does not need (re)formatting at all.
            if self.cache_hit(out_dir, &rel, &out_path, &raw) {
                continue;
            }
            to_format.push((out_path, rel, raw));
        }

        if to_format.is_empty() {
            return Ok(written);
        }

        for (out_path, _, raw) in &to_format {
            ensure_parent_dir(out_path)?;
            fs::write(tmp_format_path(out_path), raw)?;
        }
        let tmp_paths: Vec<PathBuf> = to_format.iter().map(|(p, _, _)| tmp_format_path(p)).collect();
        let out_paths: Vec<PathBuf> = to_format.iter().map(|(p, _, _)| p.clone()).collect();
        if let Err(err) = self.run_formatters_batch(&tmp_paths, &out_paths) {
            for p in &tmp_paths {
                let _ = fs::remove_file(p);
            }
            return Err(err);
        }

        for (out_path, rel, raw) in &to_format {
            let tmp_path = tmp_format_path(out_path);
            let formatted = fs::read(&tmp_path)?;
            fs::remove_file(&tmp_path)?;
            if fs::read(out_path).is_ok_and(|existing| existing == formatted) {
                // Output already correct; only the cache needed updating.
                self.cache_store(out_dir, rel, out_path, raw)?;
                continue;
            }
            fs::write(out_path, &formatted)?;
            self.cache_store(out_dir, rel, out_path, raw)?;
            written.push(out_path.clone());
        }

        Ok(written)
    }

    /// Run every configured formatter once across `tmp_paths` (parallel to
    /// `out_paths`, used only to name the batch in errors).
    ///
    /// Each formatter string is split shell-style (so `clojure -M:cljfmt fix`
    /// works directly, not just a single bare executable), and the batch's
    /// temp file paths are appended as the final arguments: `CMD [ARGS...]
    /// <file>...`. Formatters run in sequence; each sees the previous
    /// formatter's output, same as a single-file batch would.
    fn run_formatters_batch(
        &self,
        tmp_paths: &[PathBuf],
        out_paths: &[PathBuf],
    ) -> Result<(), TangleError> {
        for fmt in &self.formatters {
            let mut words = match shlex::split(fmt) {
                Some(words) if !words.is_empty() => words,
                _ => {
                    return Err(TangleError::FormatterCommandInvalid {
                        command: fmt.clone(),
                    });
                }
            };
            let program = words.remove(0);
            let status = std::process::Command::new(program)
                .args(&words)
                .args(tmp_paths)
                .status()?;
            if !status.success() {
                let paths = out_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(TangleError::FormatterFailed {
                    command: fmt.clone(),
                    path: paths,
                    status: status.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Cache lookup: true when the cached raw expansion equals `raw` and the
    /// output file's content still hashes to the value recorded at write time.
    fn cache_hit(&self, out_dir: &Path, rel: &str, out_path: &Path, raw: &[u8]) -> bool {
        let Ok(cached_raw) = fs::read(raw_cache_path(out_dir, rel)) else {
            return false;
        };
        if cached_raw != raw {
            return false;
        }
        let Ok(recorded_hash) = fs::read_to_string(hash_cache_path(out_dir, rel)) else {
            return false;
        };
        let Ok(dest) = fs::read(out_path) else {
            return false;
        };
        recorded_hash.trim() == fnv1a_hex(&dest)
    }

    /// Record the raw expansion and the hash of the written output.
    fn cache_store(
        &self,
        out_dir: &Path,
        rel: &str,
        out_path: &Path,
        raw: &[u8],
    ) -> Result<(), TangleError> {
        let raw_path = raw_cache_path(out_dir, rel);
        ensure_parent_dir(&raw_path)?;
        fs::write(&raw_path, raw)?;
        let dest = fs::read(out_path)?;
        fs::write(hash_cache_path(out_dir, rel), fnv1a_hex(&dest))?;
        Ok(())
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
            empty_chunk_name_error(chunk_name)
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

    /// Named chunks reachable from any `@file` chunk, by following chunk
    /// references transitively. `@file` chunk names are included as the
    /// traversal's roots (they are definitionally "reachable").
    ///
    /// This is the same traversal `expand_inner` performs, minus the actual
    /// expansion: only which chunks are reached, not their rendered content.
    /// It covers every document this run read into `self` with one pass,
    /// which is exactly what makes it correct across `%include`-spliced
    /// fragments too — a fragment's chunk definitions, once spliced into a
    /// main document's expanded text, are read here exactly like any other
    /// part of that document, with no separate per-document pass to miss
    /// cross-document references.
    fn reachable_chunks(&self) -> HashSet<String> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<String> = self.file_chunks.clone();
        while let Some(name) = queue.pop() {
            if !visited.insert(name.clone()) {
                continue;
            }
            if let Some(chunk) = self.chunks.get(&name) {
                for referenced in self.chunk_references(chunk) {
                    if !visited.contains(&referenced) {
                        queue.push(referenced);
                    }
                }
            }
        }
        visited
    }

    /// Every chunk name referenced anywhere across a named chunk's
    /// (possibly accumulated) definitions, in no particular order and
    /// without checking whether the name is itself defined.
    fn chunk_references(&self, chunk: &NamedChunk) -> Vec<String> {
        chunk
            .definitions
            .iter()
            .flat_map(|def| &def.content)
            .filter_map(|line| self.syntax.parse_reference_line(line))
            .map(|slot| slot.referenced_chunk.trim().to_string())
            .collect()
    }

    /// One [`TangleError::UnreferencedChunk`] per connected cluster of dead
    /// chunks — named chunks defined in this run but outside the
    /// reachability closure of every `@file` chunk — ordered by each
    /// cluster's representative's own definition site (earliest first).
    ///
    /// A dead chunk can only ever be referenced by another dead chunk: a
    /// live referrer would make it live too by definition of
    /// [`reachable_chunks`](Self::reachable_chunks). So the dead chunks
    /// form their own closed subgraph, and one forgotten reference
    /// typically kills one connected cluster of them — reported as a
    /// single error (naming the rest of the cluster for context) rather
    /// than one error per member, which would read as a wall of duplicates
    /// for the same underlying mistake.
    fn unreferenced_chunk_errors(&self) -> Vec<TangleError> {
        let live = self.reachable_chunks();
        let dead_names: HashSet<String> = self
            .chunks
            .keys()
            .filter(|name| !name.starts_with("@file ") && !live.contains(*name))
            .cloned()
            .collect();
        if dead_names.is_empty() {
            return Vec::new();
        }

        let mut out_edges: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> =
            dead_names.iter().map(|n| (n.clone(), 0)).collect();
        for name in &dead_names {
            let chunk = self
                .chunks
                .get(name)
                .expect("every dead name came from self.chunks's own keys");
            let targets: Vec<String> = self
                .chunk_references(chunk)
                .into_iter()
                .filter(|referenced| dead_names.contains(referenced))
                .collect();
            for target in &targets {
                *in_degree
                    .get_mut(target)
                    .expect("target was filtered to dead_names above") += 1;
            }
            out_edges.insert(name.clone(), targets);
        }

        let mut errors: Vec<((usize, usize), TangleError)> = group_dead_chunks(&dead_names, &out_edges)
            .into_iter()
            .map(|group| self.dead_group_error(group, &in_degree))
            .collect();
        errors.sort_by_key(|(key, _)| *key);
        errors.into_iter().map(|(_, err)| err).collect()
    }

    /// Builds one cluster's error: the representative is the member with no
    /// incoming reference from elsewhere in the cluster (the one nothing
    /// else there points at, so it is the cluster's own root cause), tying
    /// on the earliest definition site; a pure cycle with no such member
    /// (every member referenced by another member in it) falls back to the
    /// earliest-defined member outright. Returns the representative's own
    /// `(file_idx, line)` alongside the error, to order clusters by.
    fn dead_group_error(
        &self,
        mut group: Vec<String>,
        in_degree: &HashMap<String, usize>,
    ) -> ((usize, usize), TangleError) {
        group.sort();
        let representative = group
            .iter()
            .min_by_key(|name| {
                let def = &self.chunks[*name].definitions[0];
                (in_degree[*name], def.file_idx, def.line)
            })
            .expect("group_dead_chunks never returns an empty group")
            .clone();
        let def = &self.chunks[&representative].definitions[0];
        let key = (def.file_idx, def.line);
        let rest: Vec<String> = group.into_iter().filter(|n| *n != representative).collect();
        (
            key,
            TangleError::UnreferencedChunk {
                chunk: representative,
                file_name: self.file_name(def.file_idx),
                line: def.line,
                group: rest,
            },
        )
    }
}

/// Weakly-connected components of the dead-chunk subgraph: two dead chunks
/// land in the same component when either references the other, directly
/// or transitively, regardless of direction — direction only matters for
/// picking a cluster's representative afterwards, not for clustering.
fn group_dead_chunks(
    dead_names: &HashSet<String>,
    out_edges: &HashMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let mut undirected: HashMap<&str, Vec<&str>> = HashMap::new();
    for (from, targets) in out_edges {
        for to in targets {
            undirected.entry(from.as_str()).or_default().push(to.as_str());
            undirected.entry(to.as_str()).or_default().push(from.as_str());
        }
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut groups = Vec::new();
    for name in dead_names {
        if seen.contains(name.as_str()) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![name.as_str()];
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            component.push(n.to_string());
            if let Some(neighbors) = undirected.get(n) {
                stack.extend(neighbors.iter().copied());
            }
        }
        groups.push(component);
    }
    groups
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

#[cfg_attr(coverage_nightly, coverage(off))]
fn read_stdin() -> Result<String, TangleError> {
    let mut buf = String::new();
    io::stdin().lock().read_to_string(&mut buf)?;
    Ok(buf)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn read_path(path: &Path) -> Result<String, TangleError> {
    if path == Path::new("-") {
        read_stdin()
    } else {
        Ok(fs::read_to_string(path)?)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn empty_chunk_name_error(chunk_name: &str) -> Result<(), TangleError> {
    Err(TangleError::UnsafePath {
        path: chunk_name.to_string(),
    })
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn ensure_parent_dir(path: &Path) -> Result<(), TangleError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Temp file a formatter batch writes `out_path`'s raw content to and is
/// invoked against, next to the real output (so cross-filesystem renames
/// are never a concern) and distinguished by extension alone.
fn tmp_format_path(out_path: &Path) -> PathBuf {
    out_path.with_extension("reweave-fmt")
}

/// Cache file holding the raw (pre-format) expansion of an output.
fn raw_cache_path(out_dir: &Path, rel: &str) -> PathBuf {
    out_dir.join(".reweave").join(format!("{rel}.raw"))
}

/// Cache file holding the FNV-1a hash of the output at write time.
fn hash_cache_path(out_dir: &Path, rel: &str) -> PathBuf {
    out_dir.join(".reweave").join(format!("{rel}.hash"))
}

/// FNV-1a 64-bit, hex-encoded — tamper detection for the formatter cache.
fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
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
    fn validate_chunk_name_rejects_empty_named_chunks() {
        let t = Tangle::new(TangleConfig::default());

        assert!(matches!(
            t.validate_chunk_name("", false),
            Err(TangleError::UnsafePath { .. })
        ));
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
    fn write_files_skips_unchanged_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("out.txt");
        fs::write(&out, "hello\n").unwrap();
        let before = fs::metadata(&out).unwrap().modified().unwrap();
        let t = read("```text\n# <[@file out.txt]>=\nhello\n# @\n```");

        let written = t.write_files(temp.path()).unwrap();

        assert!(written.is_empty());
        assert_eq!(fs::metadata(&out).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn write_files_rewrites_changed_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("out.txt");
        fs::write(&out, "stale\n").unwrap();
        let t = read("```text\n# <[@file out.txt]>=\nfresh\n# @\n```");

        let written = t.write_files(temp.path()).unwrap();

        assert_eq!(written, vec![out.clone()]);
        assert_eq!(fs::read_to_string(&out).unwrap(), "fresh\n");
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
        let t = read("```text\n  # <[@file out.txt]>=\n# <[body]>\n# @\n# <[body]>=\nx\n# @\n```");

        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\n");
    }

    #[test]
    fn keeps_content_lines_shorter_than_definition_indent() {
        let t = read("```text\n    # <[@file out.txt]>=\nx\n# @\n```");
        assert_eq!(t.expand("@file out.txt").unwrap().join(""), "x\n");
    }

    #[test]
    fn write_files_rejects_a_chunk_defined_but_never_referenced() {
        // The real claycloj incident this check exists for: a chunk body
        // was written but the reference pulling it into its file chunk was
        // forgotten, and it silently never made it into the output.
        let temp = tempfile::tempdir().unwrap();
        let t = read(
            "```text\n# <[@file out.txt]>=\nkept\n# @\n# <[forgotten]>=\nvalue\n# @\n```",
        );

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::UnreferencedChunk { .. }));
        let message = err.to_string();
        assert!(message.contains("test.md line 5:"), "wrong location in: {message}");
        assert!(message.contains("chunk 'forgotten'"), "wrong chunk in: {message}");
        assert!(
            !message.contains("dead together with"),
            "a lone dead chunk should not list a group: {message}"
        );
        assert!(!temp.path().join("out.txt").exists());
    }

    #[test]
    fn write_files_succeeds_when_every_named_chunk_is_referenced() {
        let temp = tempfile::tempdir().unwrap();
        let t = read("```text\n# <[@file out.txt]>=\n# <[body]>\n# @\n# <[body]>=\nkept\n# @\n```");

        let written = t.write_files(temp.path()).unwrap();

        assert_eq!(written.len(), 1);
    }

    #[test]
    fn reachable_chunks_handles_a_chunk_referenced_twice_from_the_same_definition() {
        // `shared` is referenced twice in a row from the same `@file`
        // definition: both references get queued before either is popped,
        // so the second pop finds `shared` already visited — the
        // reachability scan's dedup path, not just its first-visit path.
        // `shared` must still come out live, not dead.
        let temp = tempfile::tempdir().unwrap();
        let t = read(
            "```text\n# <[@file out.txt]>=\n# <[shared]>\n# <[shared]>\n# @\n# <[shared]>=\nx\n# @\n```",
        );

        let written = t.write_files(temp.path()).unwrap();

        assert_eq!(written.len(), 1);
        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "x\nx\n"
        );
    }

    #[test]
    fn unreferenced_chunk_that_only_references_another_dead_chunk_reports_one_error() {
        // `wrapper` references `helper`, but neither is reachable from any
        // `@file` chunk: the whole two-chunk cluster is dead because of one
        // missing reference, and should surface as one error, not two.
        let temp = tempfile::tempdir().unwrap();
        let t = read(
            "```text\n# <[@file out.txt]>=\nkept\n# @\n# <[wrapper]>=\n# <[helper]>\n# @\n# <[helper]>=\nvalue\n# @\n```",
        );

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::UnreferencedChunk { .. }));
        let message = err.to_string();
        // `wrapper` is the cluster's own root cause (nothing references
        // it); `helper` is only dead as a consequence and is folded into
        // the same error instead of being reported again.
        assert!(message.contains("chunk 'wrapper'"), "wrong representative in: {message}");
        assert!(message.contains("dead together with: helper"), "helper not folded in: {message}");
    }

    #[test]
    fn unreferenced_chunk_pure_cycle_with_no_external_referrer_reports_one_error() {
        // `a` and `b` reference each other and nothing else references
        // either: every member of the cluster has an in-cluster referrer,
        // so the usual "nothing points at it" rule can't pick a
        // representative — the earliest-defined member breaks the tie.
        let temp = tempfile::tempdir().unwrap();
        let t = read(
            "```text\n# <[@file out.txt]>=\nkept\n# @\n# <[a]>=\n# <[b]>\n# @\n# <[b]>=\n# <[a]>\n# @\n```",
        );

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::UnreferencedChunk { .. }));
        let message = err.to_string();
        assert!(message.contains("chunk 'a'"), "wrong representative in: {message}");
        assert!(message.contains("dead together with: b"), "b not folded in: {message}");
    }

    #[test]
    fn unreferenced_chunk_groups_are_reported_earliest_first() {
        // Two independent dead chunks: the one defined first is the one
        // `write_files` reports (deterministic, matching how every other
        // structural error here surfaces the earliest problem first), and
        // the unrelated second one is not folded into its message.
        let temp = tempfile::tempdir().unwrap();
        let t = read(
            "```text\n# <[@file out.txt]>=\nkept\n# @\n# <[first_forgotten]>=\nx\n# @\n# <[second_forgotten]>=\ny\n# @\n```",
        );

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::UnreferencedChunk { .. }));
        let message = err.to_string();
        assert!(message.contains("chunk 'first_forgotten'"), "wrong chunk in: {message}");
        assert!(
            !message.contains("second_forgotten"),
            "unrelated dead chunk should not appear in: {message}"
        );
    }

    #[test]
    fn unreferenced_chunk_check_spans_multiple_documents_read_into_one_tangle() {
        // Mirrors reweave-cli's own architecture: every main document's
        // already macro-expanded (and %include-spliced) text is read into
        // one shared `Tangle` before `write_files` runs, so a chunk defined
        // in one document and referenced only by a dead chunk in another
        // is still correctly recognized as part of the same dead cluster,
        // with no separate per-document pass to miss the cross-document
        // reference.
        let temp = tempfile::tempdir().unwrap();
        let mut t = Tangle::new(TangleConfig::default());
        t.read(
            "```text\n# <[@file out.txt]>=\nkept\n# @\n# <[helper]>=\nvalue\n# @\n```",
            "a.md",
        );
        t.read("```text\n# <[wrapper]>=\n# <[helper]>\n# @\n```", "b.md");

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::UnreferencedChunk { .. }));
        let message = err.to_string();
        assert!(message.contains("b.md line 2:"), "wrong location in: {message}");
        assert!(message.contains("chunk 'wrapper'"), "wrong representative in: {message}");
        assert!(message.contains("dead together with: helper"), "helper not folded in: {message}");
    }
}

#[cfg(test)]
mod formatter_cache_tests {
    use super::*;

    fn counting_formatter(dir: &Path, script_name: &str, counter: &Path) -> PathBuf {
        let script = dir.join(script_name);
        fs::write(
            &script,
            format!(
                "#!/bin/sh\necho x >> {}\nprintf 'formatted\\n' >> \"$1\"\n",
                counter.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    fn count(counter: &Path) -> usize {
        fs::read_to_string(counter)
            .map(|c| c.lines().count())
            .unwrap_or(0)
    }

    fn tangle_with_formatter(fmt: &Path) -> Tangle {
        let mut t = Tangle::new(TangleConfig {
            formatters: vec![fmt.to_string_lossy().into_owned()],
            ..TangleConfig::default()
        });
        t.read("```text\n# <[@file out.txt]>=\nhello\n# @\n```", "test.md");
        t
    }

    /// A formatter that counts its own invocations (one line per run, regardless
    /// of how many file arguments it was given) and appends a marker to *every*
    /// path it receives — proving both that one run handles many files, and
    /// that each file was actually passed through, not just the first.
    fn batch_counting_formatter(dir: &Path, script_name: &str, counter: &Path) -> PathBuf {
        let script = dir.join(script_name);
        fs::write(
            &script,
            format!(
                "#!/bin/sh\necho x >> {}\nfor f in \"$@\"; do printf 'formatted\\n' >> \"$f\"; done\n",
                counter.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    #[test]
    fn cache_skips_formatter_when_nothing_changed() {
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = counting_formatter(temp.path(), "fmt.sh", &counter);
        let t = tangle_with_formatter(&fmt);

        let first = t.write_files(temp.path()).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(count(&counter), 1);

        let second = t.write_files(temp.path()).unwrap();
        assert!(second.is_empty());
        assert_eq!(count(&counter), 1, "formatter ran despite cache");

        // The cache files exist and sit in a private dir next to the outputs.
        assert!(temp.path().join(".reweave/out.txt.raw").exists());
        assert!(temp.path().join(".reweave/out.txt.hash").exists());
    }

    #[test]
    fn cache_detects_tampered_output_and_self_heals() {
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = counting_formatter(temp.path(), "fmt.sh", &counter);
        let t = tangle_with_formatter(&fmt);
        let out = temp.path().join("out.txt");

        t.write_files(temp.path()).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap(), "hello\nformatted\n");

        // Hand-edit the generated file: the dest hash no longer matches.
        fs::write(&out, "hand edited\n").unwrap();

        let written = t.write_files(temp.path()).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(count(&counter), 2, "formatter did not rerun after tamper");
        assert_eq!(fs::read_to_string(&out).unwrap(), "hello\nformatted\n");
    }

    #[test]
    fn cache_updates_after_source_change() {
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = counting_formatter(temp.path(), "fmt.sh", &counter);
        let out = temp.path().join("out.txt");

        tangle_with_formatter(&fmt).write_files(temp.path()).unwrap();

        let mut changed = Tangle::new(TangleConfig {
            formatters: vec![fmt.to_string_lossy().into_owned()],
            ..TangleConfig::default()
        });
        changed.read("```text\n# <[@file out.txt]>=\nchanged\n# @\n```", "test.md");
        changed.write_files(temp.path()).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap(), "changed\nformatted\n");
        assert_eq!(count(&counter), 2);

        // Third run with the new content: cache hit, no formatter.
        let third = changed.write_files(temp.path()).unwrap();
        assert!(third.is_empty());
        assert_eq!(count(&counter), 2);
    }

    #[test]
    fn identity_formatter_uses_raw_precheck() {
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        // Formatter that does not modify the file, only counts invocations.
        let script = temp.path().join("identity.sh");
        fs::write(
            &script,
            format!("#!/bin/sh\necho x >> {}\n", counter.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let t = tangle_with_formatter(&script);

        t.write_files(temp.path()).unwrap();
        assert_eq!(count(&counter), 1);

        let second = t.write_files(temp.path()).unwrap();
        assert!(second.is_empty());
        assert_eq!(count(&counter), 1, "identity formatter reran despite raw match");
    }

    #[test]
    fn formatter_command_carries_its_own_arguments() {
        // The formatter string is split shell-style: everything but the last word is the
        // program and its own arguments, and the temp file path is appended after them.
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("fmt-with-flag.sh");
        fs::write(
            &script,
            "#!/bin/sh\n[ \"$1\" = \"--verbose\" ] || exit 2\nprintf 'formatted\\n' >> \"$2\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut t = Tangle::new(TangleConfig {
            formatters: vec![format!("{} --verbose", script.display())],
            ..TangleConfig::default()
        });
        t.read("```text\n# <[@file out.txt]>=\nhello\n# @\n```", "test.md");

        t.write_files(temp.path()).unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "hello\nformatted\n"
        );
    }

    #[test]
    fn unparsable_formatter_command_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        let mut t = Tangle::new(TangleConfig {
            // An unterminated single quote: not splittable into shell words.
            formatters: vec!["echo 'unterminated".to_string()],
            ..TangleConfig::default()
        });
        t.read("```text\n# <[@file out.txt]>=\nhello\n# @\n```", "test.md");

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::FormatterCommandInvalid { .. }));
        assert!(err.to_string().contains("echo 'unterminated"));
    }

    #[test]
    fn blank_formatter_command_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        let mut t = Tangle::new(TangleConfig {
            // Valid shell syntax, but splits to zero words: nothing to execute.
            formatters: vec!["   ".to_string()],
            ..TangleConfig::default()
        });
        t.read("```text\n# <[@file out.txt]>=\nhello\n# @\n```", "test.md");

        let err = t.write_files(temp.path()).unwrap_err();

        assert!(matches!(err, TangleError::FormatterCommandInvalid { .. }));
        assert!(err.to_string().contains("could not be split"));
    }

    #[test]
    fn one_formatter_run_handles_every_output_in_the_batch() {
        // Reweave does not know the full set of @file outputs until every chunk
        // has been scanned, and that set can be large: the formatter must run
        // once per `write_files` call, not once per output file.
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = batch_counting_formatter(temp.path(), "fmt.sh", &counter);
        let mut t = Tangle::new(TangleConfig {
            formatters: vec![fmt.to_string_lossy().into_owned()],
            ..TangleConfig::default()
        });
        t.read(
            "```text\n# <[@file a.txt]>=\none\n# @\n# <[@file b.txt]>=\ntwo\n# @\n```",
            "test.md",
        );

        let written = t.write_files(temp.path()).unwrap();

        assert_eq!(written.len(), 2);
        assert_eq!(count(&counter), 1, "formatter ran more than once for the batch");
        assert_eq!(
            fs::read_to_string(temp.path().join("a.txt")).unwrap(),
            "one\nformatted\n"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("b.txt")).unwrap(),
            "two\nformatted\n"
        );
    }

    #[test]
    fn batch_formatter_failure_names_every_file_in_the_batch() {
        let temp = tempfile::tempdir().unwrap();
        let fmt = temp.path().join("fail.sh");
        fs::write(&fmt, "#!/bin/sh\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&fmt, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut t = Tangle::new(TangleConfig {
            formatters: vec![fmt.to_string_lossy().into_owned()],
            ..TangleConfig::default()
        });
        t.read(
            "```text\n# <[@file a.txt]>=\none\n# @\n# <[@file b.txt]>=\ntwo\n# @\n```",
            "test.md",
        );

        let err = t.write_files(temp.path()).unwrap_err();

        let message = err.to_string();
        assert!(message.contains("a.txt"), "missing a.txt in: {message}");
        assert!(message.contains("b.txt"), "missing b.txt in: {message}");
        // Failed batches leave no stray .reweave-fmt temp files behind.
        assert!(!temp.path().join("a.reweave-fmt").exists());
        assert!(!temp.path().join("b.reweave-fmt").exists());
    }

    #[test]
    fn reformatting_to_the_same_bytes_as_the_existing_output_only_updates_the_cache() {
        // The formatter normalizes every input to the same fixed output. A
        // changed raw expansion still forces a reformat (the raw cache
        // alone can't skip it), but the *formatted* result turns out
        // byte-identical to what is already on disk — the "already
        // correct" path that only needs to refresh the cache, not rewrite
        // the file or report it as written.
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("normalize.sh");
        fs::write(&script, "#!/bin/sh\nprintf 'const\\n' > \"$1\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let fmt = script.to_string_lossy().into_owned();

        let mut first_tangle = Tangle::new(TangleConfig {
            formatters: vec![fmt.clone()],
            ..TangleConfig::default()
        });
        first_tangle.read("```text\n# <[@file out.txt]>=\none\n# @\n```", "test.md");
        let first = first_tangle.write_files(temp.path()).unwrap();
        assert_eq!(first, vec![temp.path().join("out.txt")]);
        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "const\n"
        );

        let mut second_tangle = Tangle::new(TangleConfig {
            formatters: vec![fmt],
            ..TangleConfig::default()
        });
        second_tangle.read("```text\n# <[@file out.txt]>=\ntwo\n# @\n```", "test.md");
        let second = second_tangle.write_files(temp.path()).unwrap();

        assert!(second.is_empty(), "no file write was needed, only the cache");
        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "const\n"
        );
    }

    #[test]
    fn cache_hit_reformats_when_hash_cache_file_is_missing() {
        // The raw expansion is unchanged (the raw-cache half of the check
        // would pass on its own), but the hash-cache file is gone —
        // `cache_hit` must treat that as a miss rather than assume a match.
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = counting_formatter(temp.path(), "fmt.sh", &counter);
        let t = tangle_with_formatter(&fmt);

        t.write_files(temp.path()).unwrap();
        assert_eq!(count(&counter), 1);
        fs::remove_file(temp.path().join(".reweave/out.txt.hash")).unwrap();

        let second = t.write_files(temp.path()).unwrap();

        // The reformat reproduces the same bytes already on disk, so
        // nothing is (re)written — but the formatter did have to run.
        assert!(second.is_empty());
        assert_eq!(count(&counter), 2, "missing hash cache should force a reformat");
    }

    #[test]
    fn cache_hit_reformats_when_output_file_is_missing() {
        // The raw and hash caches both still match, but the generated file
        // itself was deleted (e.g. by hand, or by a cleaned output tree) —
        // `cache_hit` must notice the file is gone rather than treat a
        // matching cache as proof the output still exists.
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("count");
        let fmt = counting_formatter(temp.path(), "fmt.sh", &counter);
        let t = tangle_with_formatter(&fmt);

        t.write_files(temp.path()).unwrap();
        assert_eq!(count(&counter), 1);
        fs::remove_file(temp.path().join("out.txt")).unwrap();

        let second = t.write_files(temp.path()).unwrap();

        assert_eq!(second, vec![temp.path().join("out.txt")]);
        assert_eq!(count(&counter), 2, "missing output file should force a reformat");
        assert_eq!(
            fs::read_to_string(temp.path().join("out.txt")).unwrap(),
            "hello\nformatted\n"
        );
    }
}
