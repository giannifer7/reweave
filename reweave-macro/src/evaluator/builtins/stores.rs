use super::*;

pub(in crate::evaluator::builtins) fn builtin_pyset(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 2 {
        return Err(EvalError::InvalidUsage(
            "pyset: exactly 2 args (key, value)".into(),
        ));
    }
    let key = single_ident_param(eval, &node.parts[0], "store key")?;
    let value = eval.evaluate(&parts[1])?;
    eval.pystore_set(key, value);
    Ok("".into())
}

pub(in crate::evaluator::builtins) fn builtin_pyget(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    if node.parts.is_empty() {
        return Err(EvalError::InvalidUsage("pyget: requires a key".into()));
    }
    let key = single_ident_param(eval, &node.parts[0], "store key")?;
    Ok(eval.pystore_get(&key))
}
pub(in crate::evaluator::builtins) fn builtin_env(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    if !eval.allow_env() {
        return Err(EvalError::InvalidUsage(
            "env: environment variable access is disabled; pass --allow-env to enable".into(),
        ));
    }
    if node.parts.is_empty() {
        return Ok("".into());
    }
    let name = eval.evaluate(&node.parts[0])?;
    let lookup_name = if let Some(prefix) = eval.env_prefix() {
        format!("{prefix}{}", name.trim())
    } else {
        name.trim().to_string()
    };
    Ok(std::env::var(lookup_name).unwrap_or_default())
}
