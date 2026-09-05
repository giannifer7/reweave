mod builtins;
mod case_conversion;
mod core;
mod errors;
pub mod lexer_parser;
pub mod monty_eval;
mod state;

#[cfg(test)]
mod tests;

// Re-export everything needed by the rest of the crate
pub use crate::types::ASTNode;
pub use core::Evaluator;
pub use errors::{EvalError, EvalResult, SourceLocation};
pub use lexer_parser::lex_parse_content;
pub use monty_eval::MontyEvaluator;
pub use state::{EvalConfig, MacroDefinition, ScriptKind};
