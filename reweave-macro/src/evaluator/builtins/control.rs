use super::*;

pub(in crate::evaluator::builtins) fn builtin_if(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.is_empty() {
        eval.push_warning("%if() called with no arguments — always expands to \"\"".to_string());
        return Ok("".into());
    }
    let cond = eval.evaluate(&parts[0])?;
    if !cond.is_empty() {
        if parts.len() > 1 {
            eval.evaluate(&parts[1])
        } else {
            Ok("".into())
        }
    } else {
        if parts.len() > 2 {
            eval.evaluate(&parts[2])
        } else {
            Ok("".into())
        }
    }
}
pub(in crate::evaluator::builtins) fn builtin_match(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() < 2 {
        return Err(EvalError::InvalidUsage(
            "match: expected at least (value, default)".into(),
        ));
    }
    if !(parts.len() - 2).is_multiple_of(2) {
        return Err(EvalError::InvalidUsage(
            "match: regex/value arguments must come in pairs".into(),
        ));
    }

    let value = eval.evaluate(&parts[0])?;
    for pair in parts[2..].chunks_exact(2) {
        let pattern = eval.evaluate(&pair[0])?;
        let regex = Regex::new(&pattern).map_err(|e| {
            EvalError::BuiltinError(format!("match: invalid regex {pattern:?}: {e}"))
        })?;
        let Some(captures) = regex.captures(&value) else {
            continue;
        };

        let mut bindings = Vec::new();
        for idx in 0..captures.len() {
            if let Some(capture) = captures.get(idx) {
                bindings.push((format!("match_{idx}"), capture.as_str().to_string()));
            }
        }
        for name in regex.capture_names().flatten() {
            if let Some(capture) = captures.name(name) {
                bindings.push((name.to_string(), capture.as_str().to_string()));
            }
        }
        return eval.evaluate_with_temporary_variables(&bindings, &pair[1]);
    }

    eval.evaluate(&parts[1])
}
