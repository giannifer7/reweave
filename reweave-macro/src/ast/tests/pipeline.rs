use super::*;

#[test]
fn test_pipeline_plain_text() {
    use crate::evaluator::lex_parse_content;
    let ast = lex_parse_content("hello world", '%', 0).unwrap();
    assert_eq!(ast.kind, NodeKind::Block);
    assert_eq!(ast.parts.len(), 1);
    assert_eq!(ast.parts[0].kind, NodeKind::Text);
}

#[test]
fn test_pipeline_comments_stripped_from_ast() {
    use crate::evaluator::lex_parse_content;
    let ast = lex_parse_content("before %// comment\nafter", '%', 0).unwrap();
    fn no_comments(node: &ASTNode) {
        assert_ne!(
            node.kind,
            NodeKind::LineComment,
            "LineComment leaked into AST"
        );
        assert_ne!(
            node.kind,
            NodeKind::BlockComment,
            "BlockComment leaked into AST"
        );
        for child in &node.parts {
            no_comments(child);
        }
    }
    no_comments(&ast);
    assert!(ast.parts.iter().any(|n| n.kind == NodeKind::Text));
}

#[test]
fn test_pipeline_var_node() {
    use crate::evaluator::lex_parse_content;
    let ast = lex_parse_content("%(x)", '%', 0).unwrap();
    assert!(ast.parts.iter().any(|n| n.kind == NodeKind::Var));
}

#[test]
fn test_pipeline_macro_with_named_param() {
    use crate::evaluator::lex_parse_content;
    let ast = lex_parse_content("%foo(a, b=val)", '%', 0).unwrap();
    let mac = ast
        .parts
        .iter()
        .find(|n| n.kind == NodeKind::Macro)
        .expect("expected Macro node");
    assert_eq!(mac.parts.len(), 2);
    let unnamed = mac
        .parts
        .iter()
        .find(|p| p.name.is_none())
        .expect("unnamed param");
    let named = mac
        .parts
        .iter()
        .find(|p| p.name.is_some())
        .expect("named param");
    assert_eq!(named.name.unwrap().length, 1);
    assert!(
        unnamed
            .parts
            .iter()
            .any(|n| n.kind == NodeKind::Ident || n.kind == NodeKind::Text)
    );
}

#[test]
fn test_pipeline_tagged_block() {
    use crate::evaluator::lex_parse_content;
    let ast = lex_parse_content("%foo{ content %foo}", '%', 0).unwrap();
    let block = ast
        .parts
        .iter()
        .find(|n| n.kind == NodeKind::Block)
        .expect("expected Block node");
    assert!(!block.parts.is_empty());
}

#[test]
fn test_strip_is_idempotent() {
    use crate::Lexer;
    use crate::parser::Parser;
    let src = "hello %%// comment\nworld";
    let (tokens, _) = Lexer::new(src, '%', 0).lex();
    let li = crate::line_index::LineIndex::new(src);

    let mut parser = Parser::new();
    parser.parse(&tokens, src.as_bytes(), &li).unwrap();
    let root = 0;

    // First strip
    strip_space_before_comments(src.as_bytes(), &mut parser, root).unwrap();
    let ast1 = crate::ast::build_ast(&parser).unwrap();

    // Second strip on already-stripped parser
    strip_space_before_comments(src.as_bytes(), &mut parser, root).unwrap();
    let ast2 = crate::ast::build_ast(&parser).unwrap();

    // Both ASTs should have the same number of top-level parts
    assert_eq!(
        ast1.parts.len(),
        ast2.parts.len(),
        "strip is not idempotent"
    );
}

#[test]
fn test_ast_no_comments_invariant() {
    use crate::evaluator::lex_parse_content;
    fn check_no_comments(node: &ASTNode) {
        assert!(
            !matches!(node.kind, NodeKind::LineComment | NodeKind::BlockComment),
            "comment node {:?} leaked into AST",
            node.kind
        );
        for child in &node.parts {
            check_no_comments(child);
        }
    }
    for src in &[
        "plain text",
        "before %%// line comment\nafter",
        "before %%/* block %%*/ mid after",
        "%%def(foo, body) %%foo()",
    ] {
        let ast = lex_parse_content(src, '%', 0).unwrap();
        check_no_comments(&ast);
    }
}
