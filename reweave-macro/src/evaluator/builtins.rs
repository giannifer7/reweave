use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use regex::Regex;

use super::case_conversion::convert_case_str;
use super::core::Evaluator;
use super::errors::{EvalError, EvalResult};
use super::state::{MacroBindingKind, ScriptKind};
use crate::types::{ASTNode, NodeKind};
mod control;
mod definition;
mod include;
mod predicates;
mod scope;
mod stores;
mod strings;
mod util;

use control::{builtin_if, builtin_match};
use definition::{builtin_def, builtin_pydef, builtin_redef};
use include::{builtin_import, builtin_include};
use predicates::{builtin_eq, builtin_neq, builtin_not};
use scope::{builtin_alias, builtin_eval, builtin_export, builtin_set};
use stores::{builtin_env, builtin_pyget, builtin_pyset};
use strings::{
    builtin_capitalize, builtin_convert_case, builtin_decapitalize, builtin_to_camel_case,
    builtin_to_pascal_case, builtin_to_screaming_case, builtin_to_snake_case,
};
use util::single_ident_param;
/// Type for a builtin macro function: (Evaluator, node) -> String
pub type BuiltinFn = fn(&mut Evaluator, &ASTNode) -> EvalResult<String>;

/// Return the default builtins
pub fn default_builtins() -> HashMap<String, BuiltinFn> {
    let mut map = HashMap::new();
    map.insert("def".to_string(), builtin_def as BuiltinFn);
    map.insert("redef".to_string(), builtin_redef as BuiltinFn);
    map.insert("pydef".to_string(), builtin_pydef as BuiltinFn);
    map.insert("pyset".to_string(), builtin_pyset as BuiltinFn);
    map.insert("pyget".to_string(), builtin_pyget as BuiltinFn);
    map.insert("include".to_string(), builtin_include as BuiltinFn);
    map.insert("import".to_string(), builtin_import as BuiltinFn);
    map.insert("if".to_string(), builtin_if as BuiltinFn);
    map.insert("match".to_string(), builtin_match as BuiltinFn);
    map.insert("set".to_string(), builtin_set as BuiltinFn);
    map.insert("alias".to_string(), builtin_alias as BuiltinFn);
    map.insert("export".to_string(), builtin_export as BuiltinFn);
    map.insert("eval".to_string(), builtin_eval as BuiltinFn);
    map.insert("capitalize".to_string(), builtin_capitalize as BuiltinFn);
    map.insert(
        "decapitalize".to_string(),
        builtin_decapitalize as BuiltinFn,
    );
    map.insert(
        "convert_case".to_string(),
        builtin_convert_case as BuiltinFn,
    );
    map.insert(
        "to_snake_case".to_string(),
        builtin_to_snake_case as BuiltinFn,
    );
    map.insert(
        "to_camel_case".to_string(),
        builtin_to_camel_case as BuiltinFn,
    );
    map.insert(
        "to_pascal_case".to_string(),
        builtin_to_pascal_case as BuiltinFn,
    );
    map.insert(
        "to_screaming_case".to_string(),
        builtin_to_screaming_case as BuiltinFn,
    );
    map.insert("env".to_string(), builtin_env as BuiltinFn);
    map.insert("eq".to_string(), builtin_eq as BuiltinFn);
    map.insert("neq".to_string(), builtin_neq as BuiltinFn);
    map.insert("not".to_string(), builtin_not as BuiltinFn);
    map
}
