use super::types::{EvalOutput, ExpansionLineEntry, SourceSpan};
use crate::evaluator::state::SourceManager;
use crate::line_index::LineIndex;

/// Output accumulator that records one source span per output line.
///
/// For each completed output line the first tracked `push_str` span on that
/// line is stored.  Untracked pushes (script results, builtins) advance the line
/// counter but do not contribute a span.
///
/// This is much cheaper than recording per-push-call byte offsets: allocations
/// are proportional to line count rather than token count.
#[derive(Debug)]
pub struct TracingOutput {
    buf: String,
    /// One entry per completed output line (terminated by `\n`).
    /// `None` means that line had no tracked source span.
    line_spans: Vec<Option<SourceSpan>>,
    /// Span for the current, still-open output line.
    current_line_span: Option<SourceSpan>,
}

impl TracingOutput {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            line_spans: Vec::new(),
            current_line_span: None,
        }
    }

    fn advance_line(&mut self) {
        self.line_spans.push(self.current_line_span.take());
    }
}

impl Default for TracingOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalOutput for TracingOutput {
    fn push_str(&mut self, text: &str, span: SourceSpan) {
        if text.is_empty() {
            return;
        }
        // First tracked span on each line wins.
        if self.current_line_span.is_none() {
            self.current_line_span = Some(span.clone());
        }
        let bytes = text.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] == b'\n' {
                self.advance_line();
                // Propagate span to the next line only when more content follows
                // this '\n' within the same push_str (intermediate line of a
                // multi-line literal).  If '\n' is the last byte, leave
                // current_line_span as None so the next push_str starts fresh.
                if i + 1 < bytes.len() && self.current_line_span.is_none() {
                    self.current_line_span = Some(span.clone());
                }
            }
        }
        self.buf.push_str(text);
    }

    fn push_untracked(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        // Untracked text advances line boundaries but does not set a span.
        for b in text.bytes() {
            if b == b'\n' {
                self.advance_line();
            }
        }
        self.buf.push_str(text);
    }

    fn finish(self) -> String {
        self.buf
    }
}
impl TracingOutput {
    /// Convert the per-line span records into `ExpansionLineEntry`s suitable for
    /// storage in the line-attribution output.
    ///
    /// Returns a list of `(expanded_line_index, ExpansionLineEntry)`.
    pub fn into_line_entries(&self, sources: &SourceManager) -> Vec<(u32, ExpansionLineEntry)> {
        // Collect completed lines, plus the final open line if the output does
        // not end with a newline.
        let final_span = if !self.buf.is_empty() && !self.buf.ends_with('\n') {
            Some(self.current_line_span.as_ref())
        } else {
            None
        };

        let base = self.line_spans.iter().map(|s| s.as_ref());
        let all: Box<dyn Iterator<Item = Option<&SourceSpan>>> = match final_span {
            Some(s) => Box::new(base.chain(std::iter::once(s))),
            None => Box::new(base),
        };

        // Cache one LineIndex per source file so repeated lookups into the same
        // file are O(log n) rather than O(n).
        let mut line_index_cache: std::collections::HashMap<u32, LineIndex> =
            std::collections::HashMap::new();
        let mut results = Vec::new();
        for (line_idx, maybe_span) in all.enumerate() {
            let Some(span) = maybe_span else { continue };
            let Some(src_path) = sources.source_files().get(span.src as usize) else {
                continue;
            };
            let Some(src_bytes) = sources.get_source(span.src) else {
                continue;
            };
            let line_index = line_index_cache
                .entry(span.src)
                .or_insert_with(|| LineIndex::from_bytes(src_bytes));
            let (line_1, col_1) = line_index.line_col(span.pos);
            results.push((
                line_idx as u32,
                ExpansionLineEntry {
                    src_file: src_path.to_string_lossy().into_owned(),
                    src_line: (line_1 - 1) as u32,
                    src_col: (col_1 - 1) as u32,
                    kind: span.kind.clone(),
                },
            ));
        }
        results
    }
}
