use super::*;

#[test]
fn test_verbatim_blocks_are_opaque() {
    let input = "%[outer %macro(x) %(v) %// not a comment %]";
    let tokens = collect_tokens_with_timeout(input).expect("Failed to collect tokens");

    assert_eq!(
        tokens.first().map(|t| t.kind),
        Some(TokenKind::VerbatimOpen)
    );
    assert_eq!(
        tokens.last().map(|t| t.kind),
        Some(TokenKind::VerbatimClose)
    );
    assert!(
        tokens[1..tokens.len() - 1]
            .iter()
            .all(|t| t.kind == TokenKind::Text)
    );

    let inner = &input[2..input.len() - 2];
    let rebuilt = tokens[1..tokens.len() - 1]
        .iter()
        .map(|t| &input[t.pos..t.pos + t.length])
        .collect::<String>();
    assert_eq!(rebuilt, inner);
}

#[test]
fn test_nested_tagged_verbatim_blocks() {
    assert_tokens(
        "%py[one %inner[two%inner] three%py]",
        &[
            (TokenKind::VerbatimOpen, "%py["),
            (TokenKind::Text, "one "),
            (TokenKind::VerbatimOpen, "%inner["),
            (TokenKind::Text, "two"),
            (TokenKind::VerbatimClose, "%inner]"),
            (TokenKind::Text, " three"),
            (TokenKind::VerbatimClose, "%py]"),
        ],
    );
}
