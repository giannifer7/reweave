use super::*;

#[test]
fn test_strip_removes_space_node_before_line_comment() {
    let content = b"hello %%// comment\n";
    let mut parser = Parser::new();
    let text_idx = n(&mut parser, NodeKind::Text, 0, 5, vec![]);
    let space_idx = n(&mut parser, NodeKind::Space, 5, 1, vec![]);
    let comment_idx = n(&mut parser, NodeKind::LineComment, 6, 12, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        18,
        vec![text_idx, space_idx, comment_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    assert_eq!(root.parts.len(), 2);
    assert_eq!(root.parts[0], text_idx);
    assert_eq!(root.parts[1], comment_idx);
}

#[test]
fn test_strip_trims_trailing_spaces_in_text_before_line_comment() {
    let content = b"hello   %%// comment\n";
    let mut parser = Parser::new();
    let text_idx = n(&mut parser, NodeKind::Text, 0, 8, vec![]);
    let comment_idx = n(&mut parser, NodeKind::LineComment, 8, 12, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        20,
        vec![text_idx, comment_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    assert_eq!(root.parts.len(), 2);
    let text = parser.get_node(text_idx).unwrap();
    assert_eq!(
        text.token.length, 5,
        "trailing spaces should be stripped from text"
    );
}

#[test]
fn test_strip_removes_space_before_block_comment_followed_by_newline() {
    let content = b" %/* c %*/\nmore";
    let mut parser = Parser::new();
    let space_idx = n(&mut parser, NodeKind::Space, 0, 1, vec![]);
    let comment_idx = n(&mut parser, NodeKind::BlockComment, 1, 9, vec![]);
    let text_idx = n(&mut parser, NodeKind::Text, 11, 4, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        15,
        vec![space_idx, comment_idx, text_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    assert_eq!(root.parts.len(), 2);
    assert_eq!(root.parts[0], comment_idx);
    assert_eq!(root.parts[1], text_idx);
}

#[test]
fn test_no_strip_before_inline_block_comment() {
    let content = b" %/* c %*/ more";
    let mut parser = Parser::new();
    let space_idx = n(&mut parser, NodeKind::Space, 0, 1, vec![]);
    let comment_idx = n(&mut parser, NodeKind::BlockComment, 1, 9, vec![]);
    let text_idx = n(&mut parser, NodeKind::Text, 10, 5, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        15,
        vec![space_idx, comment_idx, text_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    assert_eq!(root.parts.len(), 3);
}

#[test]
fn test_strip_removes_multiple_spaces_before_line_comment() {
    // Text("hello") / Space / Space / LineComment — both Spaces must be removed.
    let content = b"hello  %%// comment\n";
    let mut parser = Parser::new();
    let text_idx = n(&mut parser, NodeKind::Text, 0, 5, vec![]);
    let space1_idx = n(&mut parser, NodeKind::Space, 5, 1, vec![]);
    let space2_idx = n(&mut parser, NodeKind::Space, 6, 1, vec![]);
    let comment_idx = n(&mut parser, NodeKind::LineComment, 7, 12, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        19,
        vec![text_idx, space1_idx, space2_idx, comment_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    // Both Space nodes must be gone; only Text + Comment remain.
    assert_eq!(
        root.parts.len(),
        2,
        "expected 2 parts after stripping two spaces, got {}",
        root.parts.len()
    );
    assert_eq!(root.parts[0], text_idx);
    assert_eq!(root.parts[1], comment_idx);
}

#[test]
fn test_strip_trims_trailing_tab_in_text_before_comment() {
    // `strip_ending_space` should strip tabs as well as ASCII spaces.
    // "hello\t" followed by a line comment — the tab must be stripped.
    let content = b"hello\t%%// c\n";
    let mut parser = Parser::new();
    // Text token covers "hello\t" (6 bytes), comment token covers "%%// c\n" (6 bytes).
    let text_idx = n(&mut parser, NodeKind::Text, 0, 6, vec![]);
    let comment_idx = n(&mut parser, NodeKind::LineComment, 6, 6, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        12,
        vec![text_idx, comment_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let text = parser.get_node(text_idx).unwrap();
    assert_eq!(
        text.token.length, 5,
        "trailing tab should be stripped — expected length 5, got {}",
        text.token.length
    );
}

#[test]
fn test_strip_removes_spaces_before_multiple_consecutive_comments() {
    // Text / Space / Comment1 / Space / Comment2:
    // both Space nodes must be removed (one before each comment).
    // Content layout: "text %%// c1\n %%// c2\n"
    //   0..4  "text"   (Text)
    //   4     " "      (Space1)
    //   5..11 "%%// c1" (LineComment1, ends at 11; next byte is \n at 11 but
    //                   we don't need newline accuracy for LineComment)
    //   12    " "      (Space2)
    //   13..19"%%// c2" (LineComment2)
    let content = b"text %%// c1\n %%// c2\n";
    let mut parser = Parser::new();
    let text_idx = n(&mut parser, NodeKind::Text, 0, 4, vec![]);
    let space1_idx = n(&mut parser, NodeKind::Space, 4, 1, vec![]);
    let comment1_idx = n(&mut parser, NodeKind::LineComment, 5, 7, vec![]);
    let space2_idx = n(&mut parser, NodeKind::Space, 12, 1, vec![]);
    let comment2_idx = n(&mut parser, NodeKind::LineComment, 13, 7, vec![]);
    let root_idx = n(
        &mut parser,
        NodeKind::Block,
        0,
        20,
        vec![text_idx, space1_idx, comment1_idx, space2_idx, comment2_idx],
    );
    strip_space_before_comments(content, &mut parser, root_idx).unwrap();
    let root = parser.get_node(root_idx).unwrap();
    assert_eq!(
        root.parts.len(),
        3,
        "expected [text, comment1, comment2], got {} parts",
        root.parts.len()
    );
    assert_eq!(root.parts[0], text_idx);
    assert_eq!(root.parts[1], comment1_idx);
    assert_eq!(root.parts[2], comment2_idx);
}
