use super::types::{EvalOutput, SourceSpan};

/// Fast-path output accumulator — ignores span info, just collects text.
///
/// This is functionally identical to the existing `String`-based output in
/// `Evaluator::evaluate()`.  Zero overhead: span arguments are discarded.
#[derive(Debug)]
pub struct PlainOutput {
    buf: String,
}

impl PlainOutput {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }
}

impl Default for PlainOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalOutput for PlainOutput {
    #[inline]
    fn push_str(&mut self, text: &str, _span: SourceSpan) {
        self.buf.push_str(text);
    }

    #[inline]
    fn push_untracked(&mut self, text: &str) {
        self.buf.push_str(text);
    }

    fn finish(self) -> String {
        self.buf
    }
}
