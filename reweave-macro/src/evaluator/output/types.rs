/// Indicates how a piece of output relates to the original source.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SpanKind {
    /// Literal text from the source document or a textual block.
    Literal,
    /// Text substituted from expanding a macro body.
    MacroBody { macro_name: String },
    /// Text substituted from an argument value at a macro call site.
    MacroArg {
        macro_name: String,
        param_name: String,
    },
    /// Text substituted from a global setting or without macro context.
    VarBinding { var_name: String },
    /// Text generated programmatically (e.g. Python script results, builtins)
    /// that has no direct corresponding source token for its content.
    Computed,
}
/// Byte-offset span referencing the source token that produced a piece of output.
///
/// Fields mirror `Token.src`, `Token.pos`, `Token.length` — no conversion needed.
/// Line/col can be derived on demand via `LineIndex`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceSpan {
    /// Source file index (same as `Token.src`).
    pub src: u32,
    /// Byte offset in the source string (same as `Token.pos`).
    pub pos: usize,
    /// Byte length of the span (same as `Token.length`).
    pub length: usize,
    /// The kind of expansion that produced this text.
    pub kind: SpanKind,
}
/// Generic output sink for the evaluator.
///
/// The evaluator calls `push_str` for every piece of text it produces,
/// providing the `SourceSpan` of the token that generated it.
/// `push_untracked` is used for text whose origin cannot be attributed to
/// a single source span (e.g. Python script results).
pub trait EvalOutput {
    /// Append `text` that originated at `span` in the source.
    fn push_str(&mut self, text: &str, span: SourceSpan);

    /// Append text with no span information (computed/script results).
    fn push_untracked(&mut self, text: &str);

    /// Consume the accumulator and return the rendered string.
    fn finish(self) -> String;

    /// Returns `true` for `PreciseTracingOutput`.
    /// Used to opt into per-argument span threading in `evaluate_macro_call_to`.
    fn is_tracing(&self) -> bool {
        false
    }
}
/// A serialized entry stored in the line-attribution output.
/// It maps an output line (indirectly via the table key) to the original
/// `.md` source file that generated it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpansionLineEntry {
    /// Path of the source file containing the original text.
    pub src_file: String,
    /// 0-indexed line number within the source file.
    pub src_line: u32,
    /// 0-indexed column (byte offset) within the source line.
    pub src_col: u32,
    /// The kind of macro expansion that produced this text.
    pub kind: SpanKind,
}
/// A contiguous byte range in the output attributed to one source token.
/// Gaps (script/builtin results) are absent from the list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpanRange {
    pub start: usize,
    pub end: usize,
    pub span: SourceSpan,
}
