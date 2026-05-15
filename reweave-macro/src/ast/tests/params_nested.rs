use super::*;

#[test]
fn test_param_complex_nesting() {
    let mut parser = Parser::new();
    let text1_idx = n(&mut parser, NodeKind::Text, 1, 3, vec![]);
    let var_idx = n(&mut parser, NodeKind::Var, 4, 5, vec![]);
    let space_idx = n(&mut parser, NodeKind::Space, 9, 1, vec![]);
    let macro_text_idx = n(&mut parser, NodeKind::Text, 11, 3, vec![]);
    let text2_idx = n(&mut parser, NodeKind::Text, 18, 2, vec![]);
    let macro_param_idx = n(&mut parser, NodeKind::Param, 11, 3, vec![macro_text_idx]);
    let macro_idx = n(&mut parser, NodeKind::Macro, 10, 8, vec![macro_param_idx]);
    let block_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        20,
        vec![text1_idx, var_idx, space_idx, macro_idx, text2_idx],
    );
    let param_idx = n(&mut parser, NodeKind::Param, 0, 20, vec![block_idx]);
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_none());
    assert_eq!(result.parts.len(), 1);
    let block = &result.parts[0];
    assert_eq!(block.kind, NodeKind::Block);
    assert_eq!(block.parts.len(), 5);
    check_node(&block.parts[0], NodeKind::Text, 0);
    check_node(&block.parts[1], NodeKind::Var, 0);
    check_node(&block.parts[2], NodeKind::Space, 0);
    check_node(&block.parts[3], NodeKind::Macro, 1);
    check_node(&block.parts[4], NodeKind::Text, 0);
}

#[test]
fn test_param_nested_equals() {
    let mut parser = Parser::new();
    let ident_idx = n(&mut parser, NodeKind::Ident, 0, 3, vec![]);
    let equal1_idx = n(&mut parser, NodeKind::Equal, 3, 1, vec![]);
    let text1_idx = n(&mut parser, NodeKind::Text, 4, 3, vec![]);
    let equal2_idx = n(&mut parser, NodeKind::Equal, 7, 1, vec![]);
    let text2_idx = n(&mut parser, NodeKind::Text, 8, 4, vec![]);
    let block_idx = n(
        &mut parser,
        NodeKind::Block,
        4,
        8,
        vec![text1_idx, equal2_idx, text2_idx],
    );
    let param_idx = n(
        &mut parser,
        NodeKind::Param,
        0,
        12,
        vec![ident_idx, equal1_idx, block_idx],
    );
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_some());
    assert_eq!(result.parts.len(), 1);
    let block = &result.parts[0];
    assert_eq!(block.kind, NodeKind::Block);
    assert_eq!(block.parts.len(), 3);
    check_node(&block.parts[0], NodeKind::Text, 0);
    check_node(&block.parts[1], NodeKind::Equal, 0);
    check_node(&block.parts[2], NodeKind::Text, 0);
}

#[test]
fn test_param_with_block() {
    let mut parser = Parser::new();
    let name_idx = n(&mut parser, NodeKind::Ident, 0, 3, vec![]);
    let equal_idx = n(&mut parser, NodeKind::Equal, 3, 1, vec![]);
    let text1_idx = n(&mut parser, NodeKind::Text, 5, 3, vec![]);
    let space_idx = n(&mut parser, NodeKind::Space, 8, 1, vec![]);
    let text2_idx = n(&mut parser, NodeKind::Text, 9, 4, vec![]);
    let block_idx = n(
        &mut parser,
        NodeKind::Block,
        4,
        10,
        vec![text1_idx, space_idx, text2_idx],
    );
    let param_idx = n(
        &mut parser,
        NodeKind::Param,
        0,
        14,
        vec![name_idx, equal_idx, block_idx],
    );
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_some());
    assert_eq!(result.parts.len(), 1);
    let block = &result.parts[0];
    assert_eq!(block.kind, NodeKind::Block);
    assert_eq!(block.parts.len(), 3);
    check_node(&block.parts[0], NodeKind::Text, 0);
    check_node(&block.parts[1], NodeKind::Space, 0);
    check_node(&block.parts[2], NodeKind::Text, 0);
}

#[test]
fn test_param_with_var() {
    let mut parser = Parser::new();
    let text1_idx = n(&mut parser, NodeKind::Text, 0, 3, vec![]);
    let space_idx = n(&mut parser, NodeKind::Space, 3, 1, vec![]);
    let var_idx = n(&mut parser, NodeKind::Var, 4, 5, vec![]);
    let text2_idx = n(&mut parser, NodeKind::Text, 9, 2, vec![]);
    let param_idx = n(
        &mut parser,
        NodeKind::Param,
        0,
        11,
        vec![text1_idx, space_idx, var_idx, text2_idx],
    );
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_none());
    assert_eq!(result.parts.len(), 4);
    check_node(&result.parts[0], NodeKind::Text, 0);
    check_node(&result.parts[1], NodeKind::Space, 0);
    check_node(&result.parts[2], NodeKind::Var, 0);
    check_node(&result.parts[3], NodeKind::Text, 0);
}

#[test]
fn test_param_with_nested_macro() {
    let mut parser = Parser::new();
    let name_idx = n(&mut parser, NodeKind::Ident, 0, 3, vec![]);
    let equal_idx = n(&mut parser, NodeKind::Equal, 3, 1, vec![]);
    let text_idx = n(&mut parser, NodeKind::Text, 5, 3, vec![]);
    let macro_param_idx = n(&mut parser, NodeKind::Param, 5, 3, vec![text_idx]);
    let macro_idx = n(&mut parser, NodeKind::Macro, 4, 8, vec![macro_param_idx]);
    let param_idx = n(
        &mut parser,
        NodeKind::Param,
        0,
        12,
        vec![name_idx, equal_idx, macro_idx],
    );
    let result = analyze_param(&parser, param_idx).unwrap().unwrap();
    assert!(result.name.is_some());
    assert_eq!(result.parts.len(), 1);
    check_node(&result.parts[0], NodeKind::Macro, 1);
}
