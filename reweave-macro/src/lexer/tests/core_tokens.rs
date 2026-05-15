use super::*;

#[test]
fn test_simple_completion() {
    let result = collect_tokens_with_timeout("a");
    assert!(result.is_ok());
}

#[test]
fn test_basic_tokens() {
    assert_tokens(
        "Hello %name(world)",
        &[
            (TokenKind::Text, "Hello "),
            (TokenKind::Macro, "%name("),
            (TokenKind::Ident, "world"),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_comments() {
    assert_tokens(
        "text %// line comment\nmore text",
        &[
            (TokenKind::Text, "text "),
            (TokenKind::LineComment, "%// line comment\n"),
            (TokenKind::Text, "more text"),
        ],
    );
    assert_tokens(
        "before %/* multi\nline %*/ after",
        &[
            (TokenKind::Text, "before "),
            (TokenKind::CommentOpen, "%/*"),
            (TokenKind::Text, " multi\nline "),
            (TokenKind::CommentClose, "%*/"),
            (TokenKind::Text, " after"),
        ],
    );
}

#[test]
fn test_nested_blocks() {
    assert_tokens(
        "%{outer %{inner%}%}",
        &[
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, "outer "),
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, "inner"),
            (TokenKind::BlockClose, "%}"),
            (TokenKind::BlockClose, "%}"),
        ],
    );
}
