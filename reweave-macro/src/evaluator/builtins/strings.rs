use super::*;

fn eval_first_char_case(eval: &mut Evaluator, node: &ASTNode, upper: bool) -> EvalResult<String> {
    if node.parts.is_empty() {
        return Ok("".into());
    }
    let original = eval.evaluate(&node.parts[0])?;
    if original.is_empty() {
        return Ok("".into());
    }
    let mut chars = original.chars();
    let first = if upper {
        chars.next().unwrap().to_uppercase().to_string()
    } else {
        chars.next().unwrap().to_lowercase().to_string()
    };
    Ok(format!("{}{}", first, chars.collect::<String>()))
}

pub(in crate::evaluator::builtins) fn builtin_capitalize(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    eval_first_char_case(eval, node, true)
}

pub(in crate::evaluator::builtins) fn builtin_decapitalize(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    eval_first_char_case(eval, node, false)
}
fn builtin_single_arg_case(eval: &mut Evaluator, node: &ASTNode, case: &str) -> EvalResult<String> {
    if node.parts.is_empty() {
        return Ok("".into());
    }
    let original = eval.evaluate(&node.parts[0])?;
    if original.is_empty() {
        return Ok("".into());
    }
    Ok(convert_case_str(&original, case)?)
}

pub(in crate::evaluator::builtins) fn builtin_convert_case(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let parts = &node.parts;
    if parts.len() != 2 {
        return Err(EvalError::InvalidUsage(
            "convert_case: exactly 2 args".into(),
        ));
    }
    let original = eval.evaluate(&parts[0])?;
    if original.is_empty() {
        return Ok("".into());
    }
    let case = eval.evaluate(&parts[1])?;
    Ok(convert_case_str(&original, &case)?)
}

pub(in crate::evaluator::builtins) fn builtin_to_snake_case(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    builtin_single_arg_case(eval, node, "snake")
}

pub(in crate::evaluator::builtins) fn builtin_to_camel_case(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    builtin_single_arg_case(eval, node, "camel")
}

pub(in crate::evaluator::builtins) fn builtin_to_pascal_case(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    builtin_single_arg_case(eval, node, "pascal")
}

pub(in crate::evaluator::builtins) fn builtin_to_screaming_case(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    builtin_single_arg_case(eval, node, "screaming")
}
