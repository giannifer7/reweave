use super::*;

/// `%eq(a, b)` — returns `"1"` if `a == b` (byte-exact), else `""`.
/// Canonical boolean predicate; always returns `1` or `""`, never an operand.
pub(in crate::evaluator::builtins) fn builtin_eq(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 2 {
        return Err(EvalError::InvalidUsage("eq: exactly 2 args".into()));
    }
    let a = eval.evaluate(&parts[0])?;
    let b = eval.evaluate(&parts[1])?;
    if a == b {
        Ok("1".into())
    } else {
        Ok("".into())
    }
}

/// `%neq(a, b)` — returns `"1"` if `a != b` (byte-exact), else `""`.
pub(in crate::evaluator::builtins) fn builtin_neq(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 2 {
        return Err(EvalError::InvalidUsage("neq: exactly 2 args".into()));
    }
    let a = eval.evaluate(&parts[0])?;
    let b = eval.evaluate(&parts[1])?;
    if a != b {
        Ok("1".into())
    } else {
        Ok("".into())
    }
}

/// `%not(x)` — returns `"1"` if `x` is the empty string, else `""`.
/// Logical negation: empty string is falsy, any non-empty string is truthy.
/// Accepts 0 or 1 args; 0 args is treated as empty string → returns `"1"`.
pub(in crate::evaluator::builtins) fn builtin_not(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() > 1 {
        return Err(EvalError::InvalidUsage("not: at most 1 arg".into()));
    }
    let x = if parts.is_empty() {
        String::new()
    } else {
        eval.evaluate(&parts[0])?
    };
    if x.is_empty() {
        Ok("1".into())
    } else {
        Ok("".into())
    }
}
