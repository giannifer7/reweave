// crates/reweave-macro/src/evaluator/tests/test_lexer_parser.rs
use crate::evaluator::lexer_parser::lex_parse_content;

#[test]
fn lex_parse_content_plain_text_succeeds() {
    let ast = lex_parse_content("hello world", '%', 0).unwrap();
    let _ = ast;
}

#[test]
fn lex_parse_content_empty_string_succeeds() {
    let ast = lex_parse_content("", '%', 0).unwrap();
    let _ = ast;
}

#[test]
fn lex_parse_content_unclosed_var_is_lex_error() {
    // %(foo without closing ) triggers a lex error
    let err = lex_parse_content("%(foo", '%', 0).unwrap_err();
    assert!(
        err.contains("Lexer errors"),
        "expected lex error, got: {err}"
    );
}

#[test]
fn lex_parse_content_unclosed_arg_list_is_lex_error() {
    // %foo( without closing ) triggers "Unclosed macro argument list"
    let err = lex_parse_content("%foo(bar", '%', 0).unwrap_err();
    assert!(
        err.contains("Lexer errors"),
        "expected lex error, got: {err}"
    );
}

#[test]
fn lex_parse_content_macro_call_succeeds() {
    let ast = lex_parse_content("%foo(bar, baz)", '%', 0).unwrap();
    let _ = ast;
}
