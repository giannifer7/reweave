use super::*;

fn process_include_file(eval: &mut Evaluator, node: &ASTNode) -> EvalResult<String> {
    if node.parts.is_empty() {
        return Ok("".into());
    }
    let filename = eval.evaluate(&node.parts[0])?;
    if filename.trim().is_empty() {
        return Ok("".into());
    }
    eval.do_include(&filename)
}

pub(in crate::evaluator::builtins) fn builtin_include(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    process_include_file(eval, node)
}

pub(in crate::evaluator::builtins) fn builtin_import(
    eval: &mut Evaluator,
    node: &ASTNode,
) -> EvalResult<String> {
    let _ = process_include_file(eval, node)?;
    Ok("".into())
}
