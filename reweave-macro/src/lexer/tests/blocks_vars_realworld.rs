use super::*;

#[test]
fn test_unicode_identifier_in_macro() {
    assert_tokens(
        "%macro(привет)",
        &[
            (TokenKind::Macro, "%macro("),
            (TokenKind::Text, "привет"),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_trailing_whitespace_before_comment() {
    assert_tokens(
        "%{ hi %}  %//comment\nleftover",
        &[
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, " hi "),
            (TokenKind::BlockClose, "%}"),
            (TokenKind::Text, "  "),
            (TokenKind::LineComment, "%//comment\n"),
            (TokenKind::Text, "leftover"),
        ],
    );
}

#[test]
fn test_named_block() {
    assert_tokens(
        "%blockName{ inside content %blockName}",
        &[
            (TokenKind::BlockOpen, "%blockName{"),
            (TokenKind::Text, " inside content "),
            (TokenKind::BlockClose, "%blockName}"),
        ],
    );
}

#[test]
fn test_simple_var() {
    assert_tokens("%(foo)", &[(TokenKind::Var, "%(foo)")]);
}

#[test]
fn test_var_in_block() {
    assert_tokens(
        "%{ hello %(abc) world %}",
        &[
            (TokenKind::BlockOpen, "%{"),
            (TokenKind::Text, " hello "),
            (TokenKind::Var, "%(abc)"),
            (TokenKind::Text, " world "),
            (TokenKind::BlockClose, "%}"),
        ],
    );
}

#[test]
fn test_var_in_macro() {
    assert_tokens(
        "%func( %(myVar), 123 )",
        &[
            (TokenKind::Macro, "%func("),
            (TokenKind::Space, " "),
            (TokenKind::Var, "%(myVar)"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, " "),
            (TokenKind::Text, "123"),
            (TokenKind::Space, " "),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_multiple_vars_in_text() {
    assert_tokens(
        "Here %(x) and %(y) then done",
        &[
            (TokenKind::Text, "Here "),
            (TokenKind::Var, "%(x)"),
            (TokenKind::Text, " and "),
            (TokenKind::Var, "%(y)"),
            (TokenKind::Text, " then done"),
        ],
    );
}

#[test]
fn test_incomplete_var() {
    assert_tokens(
        "%( %(abc something %( )",
        &[
            (TokenKind::Text, "%("),
            (TokenKind::Text, " "),
            (TokenKind::Text, "%(abc"),
            (TokenKind::Text, " something "),
            (TokenKind::Text, "%("),
            (TokenKind::Text, " )"),
        ],
    );
}

#[test]
fn test_real_world_macro_with_block_and_vars() {
    let input = r#"%def(shortTopCase,  case,  ch, impl, %blk{
// <[Macro_case]>=
case %(ch): {%(impl)}
// $$
%blk})"#;

    assert_tokens(
        input,
        &[
            (TokenKind::Macro, "%def("),
            (TokenKind::Ident, "shortTopCase"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, "  "),
            (TokenKind::Ident, "case"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, "  "),
            (TokenKind::Ident, "ch"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, " "),
            (TokenKind::Ident, "impl"),
            (TokenKind::Comma, ","),
            (TokenKind::Space, " "),
            (TokenKind::BlockOpen, "%blk{"),
            (TokenKind::Text, "\n// <[Macro_case]>=\ncase "),
            (TokenKind::Var, "%(ch)"),
            (TokenKind::Text, ": {"),
            (TokenKind::Var, "%(impl)"),
            (TokenKind::Text, "}\n// $$\n"),
            (TokenKind::BlockClose, "%blk}"),
            (TokenKind::CloseParen, ")"),
        ],
    );
}

#[test]
fn test_escaped_pubfunc_not_macro() {
    assert_tokens(
        "%%pubfunc(%(name), Allocator* allo, %%{",
        &[
            (TokenKind::Special, "%%"),
            (TokenKind::Text, "pubfunc("),
            (TokenKind::Var, "%(name)"),
            (TokenKind::Text, ", Allocator* allo, "),
            (TokenKind::Special, "%%"),
            (TokenKind::Text, "{"),
        ],
    );
}
