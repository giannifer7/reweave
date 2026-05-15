use super::*;

struct DefMacroConfig {
    min_params_error: String,
    name_param_context: String,
    formal_param_context: String,
    duplicate_param_error: String,
    script_kind: ScriptKind,
    binding_kind: MacroBindingKind,
    redefine: bool,
}
fn define_macro(
    eval: &mut Evaluator,
    node: &ASTNode,
    config: DefMacroConfig,
) -> EvalResult<String> {
    if node.parts.len() < 2 {
        return Err(EvalError::InvalidUsage(config.min_params_error));
    }

    let macro_name = single_ident_param(eval, &node.parts[0], &config.name_param_context)?;

    if eval.is_builtin(&macro_name) {
        return Err(EvalError::InvalidUsage(format!(
            "cannot define macro '{}': name is reserved as a built-in",
            macro_name
        )));
    }

    let body_node = node.parts.last().unwrap().clone();

    let mut seen = HashSet::new();
    let param_list = node.parts[1..(node.parts.len() - 1)].iter().try_fold(
        Vec::new(),
        |mut acc, param_node| {
            let param_name = single_ident_param(eval, param_node, &config.formal_param_context)?;
            if !seen.insert(param_name.clone()) {
                return Err(EvalError::InvalidUsage(format!(
                    "{}: parameter '{}' already used",
                    config.duplicate_param_error, param_name
                )));
            }
            acc.push(param_name);
            Ok(acc)
        },
    )?;

    let mac = crate::evaluator::state::MacroDefinition {
        name: macro_name.clone(),
        params: param_list,
        body: Arc::new(body_node),
        script_kind: config.script_kind,
        binding_kind: config.binding_kind,
        frozen_args: HashMap::new(),
    };
    if config.redefine {
        eval.redefine_macro(mac)?;
    } else {
        eval.define_macro(mac)?;
    }
    eval.record_macro_def(
        macro_name,
        node.token.src,
        node.token.pos as u32,
        (node.end_pos.saturating_sub(node.token.pos)) as u32,
    );
    Ok("".into())
}
pub(in crate::evaluator::builtins) fn builtin_def(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    define_macro(
        eval,
        node,
        DefMacroConfig {
            min_params_error: "def requires at least (name, body)".into(),
            name_param_context: "macro name".into(),
            formal_param_context: "formal parameter".into(),
            duplicate_param_error: "def".into(),
            script_kind: ScriptKind::None,
            binding_kind: MacroBindingKind::Constant,
            redefine: false,
        },
    )
}

pub(in crate::evaluator::builtins) fn builtin_redef(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    define_macro(
        eval,
        node,
        DefMacroConfig {
            min_params_error: "redef requires at least (name, body)".into(),
            name_param_context: "macro name".into(),
            formal_param_context: "formal parameter".into(),
            duplicate_param_error: "redef".into(),
            script_kind: ScriptKind::None,
            binding_kind: MacroBindingKind::Rebindable,
            redefine: true,
        },
    )
}

pub(in crate::evaluator::builtins) fn builtin_pydef(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    define_macro(
        eval,
        node,
        DefMacroConfig {
            min_params_error: "pydef requires at least (name, body)".into(),
            name_param_context: "pydef name".into(),
            formal_param_context: "pydef parameter".into(),
            duplicate_param_error: "pydef".into(),
            script_kind: ScriptKind::Python,
            binding_kind: MacroBindingKind::Constant,
            redefine: false,
        },
    )
}
