use super::*;

pub(in crate::evaluator::builtins) fn builtin_set(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 2 {
        return Err(EvalError::InvalidUsage("set: exactly 2 args".into()));
    }
    let var_name = single_ident_param(eval, &node.parts[0], "var name")?;
    let value = eval.evaluate(&parts[1])?;
    eval.set_variable(&var_name, &value);
    eval.record_var_def(
        var_name,
        node.token.src,
        node.token.pos as u32,
        (node.end_pos.saturating_sub(node.token.pos)) as u32,
    );
    Ok("".into())
}

/// `%alias(new_name, source_name[, key = val, …])` — define `new_name` as a
/// snapshot copy of the macro currently bound to `source_name`.  The first two
/// arguments must be positional plain identifiers.  Any additional arguments
/// must be named (`key = val`); their values are evaluated at alias-definition
/// time and merged into the copy's `frozen_args`, so they are in scope whenever
/// the alias is called regardless of what the enclosing scope contains at call
/// time.
///
/// This is partial application for free-variable bindings: if `source_name`
/// references `%(chunk_name)` in its body but does not declare `chunk_name` as
/// a parameter, `%alias(emit_option, source_name, chunk_name = tangle-rows)`
/// pins that free variable for the lifetime of the alias.
pub(in crate::evaluator::builtins) fn builtin_alias(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() < 2 {
        return Err(EvalError::InvalidUsage(
            "alias: at least 2 args required: alias(new_name, source_name[, key = val, …])".into(),
        ));
    }
    if parts[0].name.is_some() || parts[1].name.is_some() {
        return Err(EvalError::InvalidUsage(
            "alias: new_name and source_name must be positional, not key=val".into(),
        ));
    }
    let new_name = single_ident_param(eval, &parts[0], "alias target name")?;

    if eval.is_builtin(&new_name) {
        return Err(EvalError::InvalidUsage(format!(
            "cannot alias to '{}': name is reserved as a built-in",
            new_name
        )));
    }

    let source_name = single_ident_param(eval, &parts[1], "alias source name")?;
    let mut mac = eval.get_macro(&source_name).ok_or_else(|| {
        EvalError::InvalidUsage(format!("alias: macro '{source_name}' is not defined"))
    })?;
    mac.name = new_name.clone();
    mac.binding_kind = MacroBindingKind::Rebindable;
    for part in &parts[2..] {
        let key = match part.name.as_ref() {
            Some(tok) => eval.extract_name_value(tok),
            None => {
                return Err(EvalError::InvalidUsage(
                    "alias: override arguments must be named (key = val)".into(),
                ));
            }
        };
        let val = eval.evaluate(part)?;
        mac.frozen_args.insert(key, val);
    }
    eval.redefine_macro(mac)?;
    eval.record_macro_def(
        new_name,
        node.token.src,
        node.token.pos as u32,
        (node.end_pos.saturating_sub(node.token.pos)) as u32,
    );
    Ok("".into())
}

pub(in crate::evaluator::builtins) fn builtin_export(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 1 {
        return Err(EvalError::InvalidUsage("export: exactly 1 arg".into()));
    }
    let name = single_ident_param(eval, &node.parts[0], "var name")?;
    eval.export(&name);
    Ok("".into())
}

pub(in crate::evaluator::builtins) fn builtin_eval(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.is_empty() {
        return Err(EvalError::InvalidUsage("eval requires macroName".into()));
    }
    let macro_name = eval.evaluate(&parts[0])?;
    let macro_name = macro_name.trim();
    if macro_name.is_empty() {
        return Ok("".into());
    }
    let rest = if parts.len() > 1 {
        parts[1..].to_vec()
    } else {
        vec![]
    };
    // Use the name-argument's token so source locations in errors point to
    // the macro name in the %eval() call, not to the %eval token itself.
    let name_token = parts[0].token;
    let call_node = ASTNode {
        kind: NodeKind::Macro,
        src: name_token.src,
        token: name_token,
        end_pos: parts[0].end_pos,
        parts: rest,
        name: None,
    };
    eval.evaluate_macro_call(&call_node, macro_name)
}
