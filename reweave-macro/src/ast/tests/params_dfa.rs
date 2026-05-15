use super::*;

#[test]
fn test_param_double_equals_value_starts_with_equal() {
    // `ident = = text`: second Equal is the first value token.
    // Distinct from test_param_multiple_equals which is `ident = text = text`.
    let mut parser = Parser::new();
    let mut builder = NodeBuilder::new();
    builder.ident(3); // foo
    builder.equals(); // first =  → SeenEqual
    builder.equals(); // second = → first_good_after_equal, value starts here
    builder.text(3); // bar
    let param_idx = builder.param(&mut parser);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_some(), "should be named param");
    assert_eq!(result.name.unwrap().length, 3, "name should be 'foo'");
    // Value part list starts at the second Equal, so parts = [Equal, Text].
    assert_eq!(result.parts.len(), 2);
    check_node(&result.parts[0], NodeKind::Equal, 0);
    check_node(&result.parts[1], NodeKind::Text, 0);
}

#[test]
fn test_param_var_as_first_token_is_positional() {
    // Only Ident can start the named-detection branch; Var must produce positional.
    let mut parser = Parser::new();
    let var_idx = n(&mut parser, NodeKind::Var, 0, 5, vec![]);
    let text_idx = n(&mut parser, NodeKind::Text, 5, 3, vec![]);
    let param_idx = n(&mut parser, NodeKind::Param, 0, 8, vec![var_idx, text_idx]);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(
        result.name.is_none(),
        "Var-prefixed param should be positional"
    );
    assert_eq!(result.parts.len(), 2);
    check_node(&result.parts[0], NodeKind::Var, 0);
    check_node(&result.parts[1], NodeKind::Text, 0);
}

#[test]
fn test_param_block_as_first_token_is_positional() {
    // Block before Ident: DFA breaks immediately in Start state.
    // Even though `= text` follows, the whole param is positional.
    let mut parser = Parser::new();
    let inner_idx = n(&mut parser, NodeKind::Text, 1, 3, vec![]);
    let block_idx = n(&mut parser, NodeKind::Block, 0, 5, vec![inner_idx]);
    let equal_idx = n(&mut parser, NodeKind::Equal, 5, 1, vec![]);
    let text_idx = n(&mut parser, NodeKind::Text, 7, 3, vec![]);
    let param_idx = n(
        &mut parser,
        NodeKind::Param,
        0,
        10,
        vec![block_idx, equal_idx, text_idx],
    );
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(
        result.name.is_none(),
        "Block-prefixed param should be positional"
    );
    assert_eq!(result.parts.len(), 3);
    check_node(&result.parts[0], NodeKind::Block, 1); // block with its inner Text
    check_node(&result.parts[1], NodeKind::Equal, 0);
    check_node(&result.parts[2], NodeKind::Text, 0);
}
