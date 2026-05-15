use super::*;

#[test]
fn test_macro_with_args() {
    assert_tokens(
        "%func(a, b, c)",
        &[
            (TokenKind::Macro, "%func("),
            (TokenKind::Ident, "a"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, " "),
            (TokenKind::Ident, "b"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, " "),
            (TokenKind::Ident, "c"),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_unicode() {
    assert_tokens(
        "Hello 世界 %macro(名前)",
        &[
            (TokenKind::Text, "Hello 世界 "),
            (TokenKind::Macro, "%macro("),
            (TokenKind::Text, "名前"),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_unicode_sigil() {
    assert_tokens_with_sigil(
        "§macro(名前) §§done §/* note §*/",
        '§',
        &[
            (TokenKind::Macro, "§macro("),
            (TokenKind::Text, "名前"),
            (TokenKind::CloseParen, ")"),
            (TokenKind::Text, " "),
            (TokenKind::Special, "§§"),
            (TokenKind::Text, "done "),
            (TokenKind::CommentOpen, "§/*"),
            (TokenKind::Text, " note "),
            (TokenKind::CommentClose, "§*/"),
        ],
    );
}

#[test]
fn test_sigil_sequences() {
    assert_tokens(
        "%%double",
        &[(TokenKind::Special, "%%"), (TokenKind::Text, "double")],
    );
}

#[test]
fn test_comment_styles() {
    assert_tokens(
        "%# hash comment\n%// double slash\n%-- dash comment",
        &[
            (TokenKind::LineComment, "%# hash comment\n"),
            (TokenKind::LineComment, "%// double slash\n"),
            (TokenKind::LineComment, "%-- dash comment"),
        ],
    );
}
