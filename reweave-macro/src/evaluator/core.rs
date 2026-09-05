use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::builtins::{BuiltinFn, default_builtins};
use super::errors::{EvalError, EvalResult, SourceLocation};
use super::monty_eval::MontyEvaluator;
use super::state::{EvalConfig, EvaluatorState, MacroDefinition, ScriptKind};
use crate::types::{ASTNode, NodeKind, Token, TokenKind};

mod accessors;
mod locate;
mod do_include;
mod evaluate;
mod export;
mod extract_name;
mod macro_call;
mod node_text;
mod parse_include;
mod py_store;
mod source;
mod state_delegates;

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
