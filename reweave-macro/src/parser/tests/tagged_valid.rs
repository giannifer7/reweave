use super::*;

#[test]
fn test_empty_tagged_block() {
    assert!(lex_parse("%a{%a}").is_ok());
}

#[test]
fn test_single_char_tag() {
    assert!(lex_parse("%x{ content %x}").is_ok());
}

#[test]
fn test_tag_with_underscores_and_digits() {
    assert!(lex_parse("%block_1{ body %block_1}").is_ok());
}

#[test]
fn test_sequential_same_tag() {
    // The same tag can be reused after closing — no state leak.
    assert!(lex_parse("%foo{first%foo}%foo{second%foo}").is_ok());
}

#[test]
fn test_sequential_different_tags() {
    assert!(lex_parse("%a{x%a}%b{y%b}%c{z%c}").is_ok());
}

#[test]
fn test_three_level_nesting() {
    assert!(lex_parse("%a{ %b{ %c{ deep %c} %b} %a}").is_ok());
}

#[test]
fn test_four_level_nesting() {
    assert!(lex_parse("%a{%b{%c{%d{deep%d}%c}%b}%a}").is_ok());
}

#[test]
fn test_tagged_inside_anonymous() {
    assert!(lex_parse("%{ %foo{ inner %foo} %}").is_ok());
}

#[test]
fn test_anonymous_inside_tagged() {
    assert!(lex_parse("%foo{ %{ inner %} %foo}").is_ok());
}

#[test]
fn test_anonymous_nested_three_deep() {
    assert!(lex_parse("%{ %{ %{ deep %} %} %}").is_ok());
}

#[test]
fn test_text_before_and_after_block() {
    assert!(lex_parse("before %foo{ inside %foo} after").is_ok());
}

#[test]
fn test_var_inside_tagged_block() {
    assert!(lex_parse("%foo{ %(x) %(y) %foo}").is_ok());
}

#[test]
fn test_macro_call_inside_tagged_block() {
    assert!(lex_parse("%foo{ %bar(arg1, arg2) %foo}").is_ok());
}

#[test]
fn test_line_comment_inside_block() {
    assert!(lex_parse("%foo{\n%// this is a comment\ncontent\n%foo}").is_ok());
}

#[test]
fn test_block_comment_inside_block() {
    assert!(lex_parse("%foo{ %/* ignored %*/ content %foo}").is_ok());
}

#[test]
fn test_unicode_content_in_block() {
    // Tags are ASCII identifiers; content can be arbitrary UTF-8.
    assert!(lex_parse("%foo{ héllo wörld %foo}").is_ok());
}

#[test]
fn test_long_tag_name() {
    let src = "%very_long_tag_name_here{ body %very_long_tag_name_here}";
    assert!(lex_parse(src).is_ok());
}

#[test]
fn test_multiple_macros_across_blocks() {
    let src = "%a{ %def(x, hello) %(x) %a}%b{ %(x) %b}";
    // Just structural validity — tag matching and lex/parse pass.
    assert!(lex_parse(src).is_ok());
}
