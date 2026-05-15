mod builtins;
mod case_conversion;
mod core;
mod errors;
mod eval_api;
pub mod lexer_parser;
pub mod monty_eval;
pub mod output;
mod state;

#[cfg(test)]
mod tests;

// Re-export everything needed by the rest of the crate
pub use crate::types::ASTNode;
pub use core::Evaluator;
pub use errors::{EvalError, EvalResult};
pub use eval_api::{
    eval_file, eval_file_with_config, eval_files, eval_files_with_config, eval_string,
    eval_string_with_defaults,
};
pub use lexer_parser::lex_parse_content;
pub use monty_eval::MontyEvaluator;
pub use output::{EvalOutput, PlainOutput, PreciseTracingOutput, SourceSpan, SpanKind, SpanRange};
pub use state::{EvalConfig, MacroDefinition, ScriptKind};
