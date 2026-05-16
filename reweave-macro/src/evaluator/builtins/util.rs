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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::evaluator::EvalConfig;
    use crate::types::{ASTNode, Token, TokenKind};

    fn token(src: u32, kind: TokenKind, pos: usize, length: usize) -> Token {
        Token {
            src,
            kind,
            pos,
            length,
        }
    }

    fn node(kind: NodeKind, src: u32, token: Token, parts: Vec<ASTNode>) -> ASTNode {
        ASTNode {
            kind,
            src,
            token,
            end_pos: token.end(),
            name: None,
            parts,
        }
    }

    #[test]
    fn single_ident_param_rejects_non_param_and_empty_identifier_text() {
        let mut eval = Evaluator::new(EvalConfig::default());
        let src = eval.add_source_bytes(b"abc".to_vec(), PathBuf::from("inline.txt"));
        let ident = node(NodeKind::Ident, src, token(src, TokenKind::Ident, 99, 1), vec![]);
        let param = node(
            NodeKind::Param,
            src,
            Token::synthetic(src, 0),
            vec![ident],
        );
        let text = node(NodeKind::Text, src, token(src, TokenKind::Text, 0, 1), vec![]);

        assert!(single_ident_param(&eval, &text, "name")
            .unwrap_err()
            .to_string()
            .contains("Param node"));
        assert!(single_ident_param(&eval, &param, "name")
            .unwrap_err()
            .to_string()
            .contains("cannot be empty"));
    }

    #[test]
    fn single_ident_param_rejects_digit_starting_identifier() {
        let mut eval = Evaluator::new(EvalConfig::default());
        let src = eval.add_source_bytes(b"1abc".to_vec(), PathBuf::from("inline.txt"));
        let ident = node(NodeKind::Ident, src, token(src, TokenKind::Ident, 0, 4), vec![]);
        let param = node(
            NodeKind::Param,
            src,
            Token::synthetic(src, 0),
            vec![ident],
        );

        assert!(single_ident_param(&eval, &param, "name")
            .unwrap_err()
            .to_string()
            .contains("cannot start with a number"));
    }
}
