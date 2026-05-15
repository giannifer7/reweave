use super::types::{EvalOutput, SourceSpan, SpanRange};

/// Output accumulator with exact per-byte source attribution.
///
/// Records one `SpanRange` entry per source-token transition — far fewer
/// entries than bytes, and no granularity tradeoff.
///
/// Use `into_parts()` to obtain `(output_string, Vec<SpanRange>)`.
/// Use `span_at_byte` to query which span covers a given byte offset.
#[derive(Debug, Default)]
pub struct PreciseTracingOutput {
    buf: String,
    ranges: Vec<SpanRange>,
    current_span: Option<SourceSpan>,
    current_start: usize,
}

impl PreciseTracingOutput {
    pub fn new() -> Self {
        Self::default()
    }

    fn flush_current(&mut self) {
        if let Some(span) = self.current_span.take() {
            self.ranges.push(SpanRange {
                start: self.current_start,
                end: self.buf.len(),
                span,
            });
        }
    }

    /// Consume and return `(output_string, span_ranges)`.
    /// The ranges are sorted by `start` and cover only tracked regions.
    pub fn into_parts(mut self) -> (String, Vec<SpanRange>) {
        self.flush_current();
        (self.buf, self.ranges)
    }

    /// Return the `SourceSpan` covering `byte_offset`, or `None` for untracked gaps.
    pub fn span_at_byte(ranges: &[SpanRange], byte_offset: usize) -> Option<&SourceSpan> {
        let idx = ranges.partition_point(|sr| sr.start <= byte_offset);
        if idx == 0 {
            return None;
        }
        let sr = &ranges[idx - 1];
        if byte_offset < sr.end {
            Some(&sr.span)
        } else {
            None
        }
    }
}

impl EvalOutput for PreciseTracingOutput {
    fn is_tracing(&self) -> bool {
        true
    }

    fn push_str(&mut self, text: &str, span: SourceSpan) {
        if text.is_empty() {
            return;
        }
        let same = self
            .current_span
            .as_ref()
            .is_some_and(|s| s.src == span.src && s.pos == span.pos && s.length == span.length);
        if !same {
            self.flush_current();
            self.current_start = self.buf.len();
            self.current_span = Some(span);
        }
        self.buf.push_str(text);
    }

    fn push_untracked(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.flush_current(); // end current span; gap follows
        self.buf.push_str(text);
    }

    fn finish(self) -> String {
        self.into_parts().0
    }
}
