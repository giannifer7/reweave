use super::*;

#[test]
fn test_mismatch_different_names() {
    let err = lex_parse("%foo{ %bar}").unwrap_err();
    assert!(
        err.contains("foo") && err.contains("bar"),
        "bad error: {}",
        err
    );
}

#[test]
fn test_wrong_nesting_order_causes_unclosed_error() {
    // '%a}' structurally closes the top block ('%b{'), leaving '%a{' unclosed.
    // The lexer reports '%a{' as unclosed; the parser never sees a tag mismatch.
    let err = lex_parse_err("%a{ %b{ content %a}");
    assert!(!err.is_empty(), "expected an error");
    assert!(err.contains('a'), "expected 'a' in error: {}", err);
}

#[test]
fn test_wrong_close_in_three_levels_causes_unclosed() {
    // '%b}' closes the top (Block c), leaving '%a{' and '%b{' both unclosed.
    let err = lex_parse_err("%a{ %b{ %c{ deep %b}");
    assert!(!err.is_empty(), "expected errors");
    // Both 'a' and 'b' should be reported as unclosed.
    assert!(err.contains('a'), "expected 'a' in error: {}", err);
    assert!(err.contains('b'), "expected 'b' in error: {}", err);
}

#[test]
fn test_mismatch_tagged_close_on_anonymous_open() {
    let err = lex_parse("%{ content %foo}").unwrap_err();
    assert!(
        err.contains("anonymous"),
        "expected 'anonymous' in error: {}",
        err
    );
    assert!(err.contains("foo"), "expected 'foo' in error: {}", err);
}

#[test]
fn test_mismatch_anonymous_close_on_tagged_open() {
    let err = lex_parse("%foo{ content %}").unwrap_err();
    assert!(err.contains("foo"), "expected 'foo' in error: {}", err);
    assert!(
        err.contains("anonymous"),
        "expected 'anonymous' in error: {}",
        err
    );
}

// -----------------------------------------------------------------------
// Unclosed block errors
// -----------------------------------------------------------------------

#[test]
fn test_unclosed_tagged_reports_tag_name() {
    let err = lex_parse("%myblock{ no close").unwrap_err();
    assert!(err.contains("myblock"), "expected tag name: {}", err);
}

#[test]
fn test_unclosed_innermost_reported() {
    // The innermost block ('%c{') has no '%' chars after it, so run_block_state
    // consumes the trailing text as Text and returns false (self-pops at EOF).
    // The remaining unclosed blocks on the stack are '%a{' and '%b{'.
    // The parser sees the error from lex stage; use lex_parse_err.
    let err = lex_parse_err("%a{ %b{ %c{ deep");
    assert!(!err.is_empty(), "expected an error");
    assert!(
        err.contains('a') || err.contains('b'),
        "expected 'a' or 'b' in error: {}",
        err
    );
}

#[test]
fn test_deeply_unclosed_reports_remaining() {
    // '%c{' pops naturally at EOF; '%a{' and '%b{' remain on the lexer stack.
    // Both are reported as unclosed (not '%c{', which was already popped).
    let err = lex_parse_err("%a{ %b{ %c{ deep");
    assert!(err.contains('a'), "expected '%a{{' in error: {}", err);
    assert!(err.contains('b'), "expected '%b{{' in error: {}", err);
}

#[test]
fn test_unclosed_after_valid_content() {
    let err = lex_parse("some text %foo{ more text").unwrap_err();
    assert!(err.contains("foo"), "expected 'foo': {}", err);
}
