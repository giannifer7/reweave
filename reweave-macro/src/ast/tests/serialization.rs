use super::*;

#[test]
fn test_serialize_bfs_child_indices() {
    // Tree: Root[A, B], A[C, D], B[], C[], D[]
    // With the old DFS traversal B landed at index 4 instead of 2.
    // BFS guarantees: Root→[1,2], A→[3,4], B/C/D are leaves at 2/3/4.
    let tok = |pos| Token {
        src: 0,
        kind: TokenKind::Text,
        pos,
        length: 1,
    };
    let c = ASTNode {
        kind: NodeKind::Text,
        src: 0,
        token: tok(3),
        end_pos: 4,
        parts: vec![],
        name: None,
    };
    let d = ASTNode {
        kind: NodeKind::Text,
        src: 0,
        token: tok(4),
        end_pos: 5,
        parts: vec![],
        name: None,
    };
    let b = ASTNode {
        kind: NodeKind::Text,
        src: 0,
        token: tok(2),
        end_pos: 3,
        parts: vec![],
        name: None,
    };
    let a = ASTNode {
        kind: NodeKind::Macro,
        src: 0,
        token: tok(1),
        end_pos: 6,
        parts: vec![c, d],
        name: None,
    };
    let root = ASTNode {
        kind: NodeKind::Block,
        src: 0,
        token: tok(0),
        end_pos: 7,
        parts: vec![a, b],
        name: None,
    };
    let nodes = serialize_ast_nodes(&root);
    assert_eq!(nodes.len(), 5);
    assert!(nodes[0].contains("[1,2]"), "root children: {}", nodes[0]);
    assert!(nodes[1].contains("[3,4]"), "A children: {}", nodes[1]);
    assert!(nodes[2].ends_with(",[]]"), "B should be leaf: {}", nodes[2]);
    assert!(nodes[3].ends_with(",[]]"), "C should be leaf: {}", nodes[3]);
    assert!(nodes[4].ends_with(",[]]"), "D should be leaf: {}", nodes[4]);
}

#[test]
fn test_serialize_bfs_deep_linear_chain() {
    // A → B → C → D (linear, each node has exactly one child).
    // BFS guarantees: A's child is at index 1, B's at 2, C's at 3, D is a leaf.
    let tok = |pos| Token {
        src: 0,
        kind: TokenKind::Text,
        pos,
        length: 1,
    };
    let d = ASTNode {
        kind: NodeKind::Text,
        src: 0,
        token: tok(3),
        end_pos: 4,
        parts: vec![],
        name: None,
    };
    let c = ASTNode {
        kind: NodeKind::Macro,
        src: 0,
        token: tok(2),
        end_pos: 4,
        parts: vec![d],
        name: None,
    };
    let b = ASTNode {
        kind: NodeKind::Macro,
        src: 0,
        token: tok(1),
        end_pos: 4,
        parts: vec![c],
        name: None,
    };
    let a = ASTNode {
        kind: NodeKind::Block,
        src: 0,
        token: tok(0),
        end_pos: 4,
        parts: vec![b],
        name: None,
    };
    let nodes = serialize_ast_nodes(&a);
    assert_eq!(nodes.len(), 4);
    assert!(
        nodes[0].contains("[1]"),
        "A should point to child at 1: {}",
        nodes[0]
    );
    assert!(
        nodes[1].contains("[2]"),
        "B should point to child at 2: {}",
        nodes[1]
    );
    assert!(
        nodes[2].contains("[3]"),
        "C should point to child at 3: {}",
        nodes[2]
    );
    assert!(
        nodes[3].ends_with(",[]]"),
        "D should be a leaf: {}",
        nodes[3]
    );
}

#[test]
fn test_serialize_token_src_field_present() {
    // token.src must appear in the output so external evaluators can trace
    // which source file a node came from.
    let root = ASTNode {
        kind: NodeKind::Block,
        src: 0,
        token: Token {
            src: 2,
            kind: TokenKind::Text,
            pos: 5,
            length: 3,
        },
        end_pos: 8,
        parts: vec![],
        name: None,
    };
    let nodes = serialize_ast_nodes(&root);
    assert_eq!(nodes.len(), 1);
    assert!(
        nodes[0].starts_with("[10,2,"),
        "token src not present: {}",
        nodes[0]
    );
}
