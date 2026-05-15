use super::*;

#[test]
fn test_error_cases() {
    assert_tokens(
        "%{incomplete",
        &[
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, "incomplete"),
        ],
    );
    assert_tokens(
        "%macro(incomplete",
        &[
            (TokenKind::Macro, "%macro("),
            (TokenKind::Ident, "incomplete"),
        ],
    );
    assert_tokens(
        "%/* unfinished",
        &[
            (TokenKind::CommentOpen, "%/*"),
            (TokenKind::Text, " unfinished"),
        ],
    );
}

#[test]
fn test_bare_percent_inside_comment() {
    assert_tokens(
        "%/* 100% done %*/",
        &[
            (TokenKind::CommentOpen, "%/*"),
            (TokenKind::Text, " 100% done "),
            (TokenKind::CommentClose, "%*/"),
        ],
    );
    let (_, errors) = Lexer::new("%/* 100% done %*/", '%', 0).lex();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}

#[test]
fn test_nested_comment() {
    let input = "%/* outer comment %/* inner %*/ outer %*/";
    assert_tokens(
        input,
        &[
            (TokenKind::CommentOpen, "%/*"),
            (TokenKind::Text, " outer comment "),
            (TokenKind::CommentOpen, "%/*"),
            (TokenKind::Text, " inner "),
            (TokenKind::CommentClose, "%*/"),
            (TokenKind::Text, " outer "),
            (TokenKind::CommentClose, "%*/"),
        ],
    );
}

#[test]
fn test_unfinished_sigil() {
    assert_tokens("%something", &[(TokenKind::Text, "%something")]);
}

#[test]
fn test_percent_identifier_no_error() {
    let (_, errors) = Lexer::new("%something", '%', 0).lex();
    assert!(
        errors.is_empty(),
        "expected no errors for %identifier, got: {:?}",
        errors
    );
}

#[test]
fn test_percent_identifier_mid_text() {
    assert_tokens(
        "%something more text",
        &[
            (TokenKind::Text, "%something"),
            (TokenKind::Text, " more text"),
        ],
    );
}

#[test]
fn test_printf_format_specifiers() {
    let input = "%d %s %f";
    let (_, errors) = Lexer::new(input, '%', 0).lex();
    assert!(
        errors.is_empty(),
        "expected no errors for printf specifiers, got: {:?}",
        errors
    );
    assert_tokens(
        input,
        &[
            (TokenKind::Text, "%d"),
            (TokenKind::Text, " "),
            (TokenKind::Text, "%s"),
            (TokenKind::Text, " "),
            (TokenKind::Text, "%f"),
        ],
    );
}

#[test]
fn test_unmatched_closing_blocks_report_errors() {
    let (_, errors) = Lexer::new("%} %tag}", '%', 0).lex();
    let messages = errors
        .iter()
        .map(|err| err.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("Unmatched block close"))
    );

    let (_, errors) = Lexer::new("%] %tag]", '%', 0).lex();
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unmatched verbatim close"))
    );
}

#[test]
fn test_invalid_comment_prefixes_report_errors() {
    let (_, errors) = Lexer::new("%/x %-x", '%', 0).lex();
    let text = errors
        .iter()
        .map(|err| err.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("Unexpected char after '%/'"));
    assert!(text.contains("Unexpected char after '%-'"));
}

#[test]
fn test_hash_and_dash_line_comments() {
    assert_tokens(
        "%# hash\n%-- dash\n",
        &[
            (TokenKind::LineComment, "%# hash\n"),
            (TokenKind::LineComment, "%-- dash\n"),
        ],
    );
}

#[test]
fn test_sigil_at_eof_and_unrecognized_char() {
    assert_tokens("%", &[(TokenKind::Text, "%")]);

    let (_, errors) = Lexer::new("%@", '%', 0).lex();
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unrecognized char"))
    );
}
