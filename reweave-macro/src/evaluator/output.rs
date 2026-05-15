mod plain;
#[cfg_attr(coverage_nightly, coverage(off))]
mod precise;
#[cfg_attr(coverage_nightly, coverage(off))]
mod tracing;
mod types;

pub use plain::PlainOutput;
pub use precise::PreciseTracingOutput;
pub use tracing::TracingOutput;
pub use types::{EvalOutput, ExpansionLineEntry, SourceSpan, SpanKind, SpanRange};
