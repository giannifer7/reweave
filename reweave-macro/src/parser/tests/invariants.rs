use super::*;

#[test]
fn test_eof_token_not_in_ast() {
    use crate::types::NodeKind;
    let src = "hello";
    let (tokens, _) = Lexer::new(src, '%', 0).lex();
    let mut parser = Parser::new();
    parser
        .parse(&tokens, src.as_bytes(), &LineIndex::new(src))
        .unwrap();
    // Walk all nodes — none should have NodeKind::Text with length 0 from EOF.
    // More directly: the EOF token has length 0 and kind EOF.
    // If it were added as Text it would appear as a zero-length Text child of root.
    let root = parser.get_node(0).unwrap();
    for &child_idx in &root.parts {
        let child = parser.get_node(child_idx).unwrap();
        // EOF token (length 0, kind Text) must not appear
        assert!(
            !(child.kind == NodeKind::Text
                && child.token.length == 0
                && child.token.pos == src.len()),
            "EOF token leaked into AST as Text node"
        );
    }
}

// -----------------------------------------------------------------------
// Root block end_pos is set correctly
// -----------------------------------------------------------------------

#[test]
fn test_root_end_pos_set() {
    let src = "hello world";
    let (tokens, _) = Lexer::new(src, '%', 0).lex();
    let mut parser = Parser::new();
    parser
        .parse(&tokens, src.as_bytes(), &LineIndex::new(src))
        .unwrap();
    let root = parser.get_node(0).unwrap();
    assert_eq!(
        root.end_pos,
        src.len(),
        "root end_pos should equal input length"
    );
}

#[test]
fn test_root_end_pos_with_block() {
    let src = "%foo{ content %foo}";
    let (tokens, _) = Lexer::new(src, '%', 0).lex();
    let mut parser = Parser::new();
    parser
        .parse(&tokens, src.as_bytes(), &LineIndex::new(src))
        .unwrap();
    let root = parser.get_node(0).unwrap();
    assert_eq!(root.end_pos, src.len());
}
