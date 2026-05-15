use super::*;

/// Helper: Checks that a Param node contains exactly one identifier child
pub(in crate::evaluator::builtins) fn single_ident_param(
    eval: &Evaluator,
    param_node: &ASTNode,
    desc: &str,
) -> EvalResult<String> {
    if param_node.kind != NodeKind::Param {
        return Err(EvalError::InvalidUsage(format!(
            "{desc} must be a Param node"
        )));
    }

    // If there's a name property, this was an equals-style param
    if param_node.name.is_some() {
        return Err(EvalError::InvalidUsage(format!(
            "{desc} must be a single identifier (found an '=' style param?)"
        )));
    }

    // Filter out comments and spaces
    let nonspace: Vec<_> = param_node
        .parts
        .iter()
        .filter(|child| {
            !matches!(
                child.kind,
                NodeKind::Space | NodeKind::LineComment | NodeKind::BlockComment
            )
        })
        .collect();

    if nonspace.len() != 1 {
        return Err(EvalError::InvalidUsage(format!(
            "{desc} must be a single identifier"
        )));
    }

    let ident_node = &nonspace[0];
    if ident_node.kind != NodeKind::Ident {
        return Err(EvalError::InvalidUsage(format!(
            "{desc} must be a single identifier"
        )));
    }

    let text = eval.node_text(ident_node).trim().to_string();
    if text.is_empty() {
        return Err(EvalError::InvalidUsage(format!("{desc} cannot be empty")));
    }

    // Check that identifier doesn't start with a number
    if text.chars().next().unwrap().is_ascii_digit() {
        return Err(EvalError::InvalidUsage(format!(
            "{desc} cannot start with a number"
        )));
    }

    Ok(text)
}
