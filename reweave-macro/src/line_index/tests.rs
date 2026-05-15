use super::LineIndex;

#[test]
fn test_single_line() {
    let idx = LineIndex::new("hello");
    assert_eq!(idx.line_col(0), (1, 1));
    assert_eq!(idx.line_col(4), (1, 5));
}

#[test]
fn test_two_lines() {
    // "ab\ncd"  — newline at offset 2
    let idx = LineIndex::new("ab\ncd");
    assert_eq!(idx.line_col(0), (1, 1)); // 'a'
    assert_eq!(idx.line_col(1), (1, 2)); // 'b'
    assert_eq!(idx.line_col(2), (1, 3)); // '\n'
    assert_eq!(idx.line_col(3), (2, 1)); // 'c'
    assert_eq!(idx.line_col(4), (2, 2)); // 'd'
}

#[test]
fn test_three_lines() {
    let idx = LineIndex::new("a\nb\nc");
    assert_eq!(idx.line_col(0), (1, 1));
    assert_eq!(idx.line_col(2), (2, 1));
    assert_eq!(idx.line_col(4), (3, 1));
}
