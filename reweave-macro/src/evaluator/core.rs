use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::builtins::{BuiltinFn, default_builtins};
use super::errors::{EvalError, EvalResult};
use super::monty_eval::MontyEvaluator;
use super::output::{EvalOutput, PreciseTracingOutput, SourceSpan, SpanKind, SpanRange};
use super::state::{EvalConfig, EvaluatorState, MacroDefinition, ScriptKind, TrackedValue};
use crate::types::{ASTNode, NodeKind, Token, TokenKind};

mod accessors;
mod do_include;
mod evaluate;
mod evaluate_to;
mod export;
mod extract_name;
mod macro_call;
mod macro_call_to;
mod node_text;
mod parse_include;
mod py_store;
mod source;
mod state_delegates;
mod tracing;

pub struct Evaluator {
    state: EvaluatorState,
    builtins: HashMap<String, BuiltinFn>,
    monty_evaluator: MontyEvaluator,
    py_store: HashMap<String, String>,
}

#[derive(Clone, Copy)]
struct PositionalBinding<'a> {
    param_name: &'a str,
    param_node: &'a ASTNode,
}

#[derive(Clone)]
struct NamedBinding<'a> {
    arg_name: String,
    param_node: &'a ASTNode,
}

struct BindingPlan<'a> {
    positional: Vec<PositionalBinding<'a>>,
    named: Vec<NamedBinding<'a>>,
    unbound: Vec<&'a str>,
}
