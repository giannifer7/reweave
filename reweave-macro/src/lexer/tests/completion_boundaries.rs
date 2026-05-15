use super::*;

#[test]
fn test_lexer_completion() {
    assert_tokens("", &[]);
    assert_tokens("a", &[(TokenKind::Text, "a")]);
    assert_tokens(
        "text%",
        &[(TokenKind::Text, "text"), (TokenKind::Text, "%")],
    );
    assert_tokens(
        "text %",
        &[(TokenKind::Text, "text "), (TokenKind::Text, "%")],
    );
}

#[test]
fn test_lexer_buffer_boundaries() {
    assert_tokens(
        "%token( rest",
        &[
            (TokenKind::Macro, "%token("),
            (TokenKind::Space, " "),
            (TokenKind::Ident, "rest"),
        ],
    );
    assert_tokens(
        "start %token(",
        &[(TokenKind::Text, "start "), (TokenKind::Macro, "%token(")],
    );
    assert_tokens(
        " % ",
        &[
            (TokenKind::Text, " "),
            (TokenKind::Text, "%"),
            (TokenKind::Text, " "),
        ],
    );
}

#[test]
fn test_leading_trailing_spaces() {
    assert_tokens("   Hello   ", &[(TokenKind::Text, "   Hello   ")]);
}

#[test]
fn test_macro_without_arguments() {
    assert_tokens(
        "%macro()",
        &[(TokenKind::Macro, "%macro("), (TokenKind::CloseParen, ")")],
    );
}

#[test]
fn test_comment_immediately_following_block() {
    assert_tokens(
        "%{ hi %}%//comment\nleftover",
        &[
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, " hi "),
            (TokenKind::BlockClose, "%}"),
            (TokenKind::LineComment, "%//comment\n"),
            (TokenKind::Text, "leftover"),
        ],
    );
}

#[test]
fn test_multiple_unmatched_percents() {
    assert_tokens(
        "text % some % more",
        &[
            (TokenKind::Text, "text "),
            (TokenKind::Text, "%"),
            (TokenKind::Text, " some "),
            (TokenKind::Text, "%"),
            (TokenKind::Text, " more"),
        ],
    );
}
