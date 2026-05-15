use super::*;

#[test]
fn test_param_identifier_only() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.space(1);
    builder.ident(3);
    builder.space(1);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 2);
    check_node(&result.parts[0], NodeKind::Ident, 0);
    check_node(&result.parts[1], NodeKind::Space, 0);
}

#[test]
fn test_empty_param() {
    // analyze_param returns Some for empty param; trailing-empty trimming
    // happens at Macro level via trim_trailing_empty_params.
    let mut parser = Parser::new();
    let builder = NodeBuilder::new();
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().parts.is_empty());
}

#[test]
fn test_param_with_comments() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.comment(1);
    builder.ident(3);
    builder.comment(1);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 1);
    check_node(&result.parts[0], NodeKind::Ident, 0);
    assert_eq!(result.parts[0].token.pos, 1);
    assert_eq!(result.parts[0].token.length, 3);
}

#[test]
fn test_param_value_only() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.space(1);
    builder.text(3);
    builder.space(1);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 2);
    assert!(result.name.is_none());
    check_node(&result.parts[0], NodeKind::Text, 0);
    check_node(&result.parts[1], NodeKind::Space, 0);
}

#[test]
fn test_param_name_equals_value() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3);
    builder.equals();
    builder.text(4);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 1);
    assert!(result.name.is_some());
    check_node(&result.parts[0], NodeKind::Text, 0);
    let name = result.name.unwrap();
    assert_eq!(name.pos, 0);
    assert_eq!(name.length, 3);
}

#[test]
fn test_param_equals_without_ident() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.equals();
    builder.text(4);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 2);
    assert!(result.name.is_none());
    check_node(&result.parts[0], NodeKind::Equal, 0);
    check_node(&result.parts[1], NodeKind::Text, 0);
}

#[test]
fn test_param_equals_with_blank() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3);
    builder.equals();
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 0);
    assert!(result.name.is_some());
    let name = result.name.unwrap();
    assert_eq!(name.pos, 0);
    assert_eq!(name.length, 3);
}

#[test]
fn test_param_equals_only_comment() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3);
    builder.equals();
    builder.comment(5);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 0);
    assert!(result.name.is_some());
    let name = result.name.unwrap();
    assert_eq!(name.pos, 0);
    assert_eq!(name.length, 3);
}

#[test]
fn test_trailing_comma_empty_param_is_ignored() {
    // analyze_param preserves empty params; trailing-empty trimming
    // at Macro level removes them from the Macro node's parts.
    let mut parser = Parser::new();
    let builder = NodeBuilder::new();
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap();
    assert!(result.is_some());
}

#[test]
fn test_param_complex_spacing() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.space(2);
    builder.comment(4);
    builder.space(1);
    builder.ident(3);
    builder.space(2);
    builder.comment(5);
    builder.equals();
    builder.space(3);
    builder.comment(4);
    builder.text(4);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    check_node(&result, NodeKind::Param, 1);
    assert!(result.name.is_some());
    check_node(&result.parts[0], NodeKind::Text, 0);
}

#[test]
fn test_param_multiple_equals() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3);
    builder.equals();
    builder.text(2);
    builder.equals();
    builder.text(2);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_some());
    assert_eq!(result.parts.len(), 3);
    check_node(&result.parts[0], NodeKind::Text, 0);
    check_node(&result.parts[1], NodeKind::Equal, 0);
    check_node(&result.parts[2], NodeKind::Text, 0);
}

#[test]
fn test_param_multiple_idents() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3);
    builder.space(1);
    builder.ident(3);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_none());
    assert_eq!(result.parts.len(), 3);
    check_node(&result.parts[0], NodeKind::Ident, 0);
    check_node(&result.parts[1], NodeKind::Space, 0);
    check_node(&result.parts[2], NodeKind::Ident, 0);
}

#[test]
fn test_param_mixed_content() {
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.text(2);
    builder.space(1);
    builder.ident(3);
    builder.equals();
    builder.text(2);
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_none());
    assert_eq!(result.parts.len(), 5);
    check_node(&result.parts[0], NodeKind::Text, 0);
    check_node(&result.parts[1], NodeKind::Space, 0);
    check_node(&result.parts[2], NodeKind::Ident, 0);
    check_node(&result.parts[3], NodeKind::Equal, 0);
    check_node(&result.parts[4], NodeKind::Text, 0);
}
