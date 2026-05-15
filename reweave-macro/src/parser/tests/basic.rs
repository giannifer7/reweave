use super::*;

#[test]
fn test_tagged_block_match() {
    assert!(lex_parse("%foo{ content %foo}").is_ok());
}

#[test]
fn test_anonymous_block_match() {
    assert!(lex_parse("%{ content %}").is_ok());
}

#[test]
fn test_tagged_block_mismatch() {
    let err = lex_parse("%foo{ content %bar}").unwrap_err();
    assert!(err.contains("foo"), "expected 'foo' in error: {}", err);
    assert!(err.contains("bar"), "expected 'bar' in error: {}", err);
}

#[test]
fn test_tagged_vs_anonymous_mismatch() {
    let err = lex_parse("%foo{ content %}").unwrap_err();
    assert!(err.contains("foo"), "expected 'foo' in error: {}", err);
    assert!(
        err.contains("anonymous"),
        "expected 'anonymous' in error: {}",
        err
    );
}

#[test]
fn test_anonymous_vs_tagged_mismatch() {
    let err = lex_parse("%{ content %foo}").unwrap_err();
    assert!(err.contains("foo"), "expected 'foo' in error: {}", err);
    assert!(
        err.contains("anonymous"),
        "expected 'anonymous' in error: {}",
        err
    );
}

#[test]
fn test_nested_tagged_blocks_match() {
    assert!(lex_parse("%outer{ %inner{ content %inner} %outer}").is_ok());
}

#[test]
fn test_unclosed_tagged_block() {
    let err = lex_parse("%foo{ no close").unwrap_err();
    assert!(err.contains("foo"), "expected tag name in error: {}", err);
}

#[test]
fn test_unclosed_anonymous_block() {
    let err = lex_parse("%{ no close").unwrap_err();
    assert!(
        err.contains("anonymous"),
        "expected 'anonymous' in error: {}",
        err
    );
}

#[test]
fn test_basic_parsing() {
    let tokens = vec![
        Token {
            src: 0,
            kind: TokenKind::Text,
            pos: 0,
            length: 5,
        },
        Token {
            src: 0,
            kind: TokenKind::BlockOpen,
            pos: 5,
            length: 2,
        },
        Token {
            src: 0,
            kind: TokenKind::Text,
            pos: 7,
            length: 3,
        },
        Token {
            src: 0,
            kind: TokenKind::BlockClose,
            pos: 10,
            length: 2,
        },
    ];

    let mut parser = Parser::new();
    assert!(
        parser
            .parse(&tokens, &[], &LineIndex::from_bytes(&[]))
            .is_ok()
    );

    let json = parser.to_json();
    assert!(!json.is_empty());
}

#[test]
fn test_empty_input() {
    let tokens = vec![];
    let mut parser = Parser::new();
    assert!(
        parser
            .parse(&tokens, &[], &LineIndex::from_bytes(&[]))
            .is_ok()
    );
}

#[test]
fn test_macro_parsing() {
    let tokens = vec![
        Token {
            src: 0,
            kind: TokenKind::Macro,
            pos: 0,
            length: 2,
        },
        Token {
            src: 0,
            kind: TokenKind::Ident,
            pos: 2,
            length: 4,
        },
        Token {
            src: 0,
            kind: TokenKind::Space,
            pos: 6,
            length: 1,
        },
        Token {
            src: 0,
            kind: TokenKind::Equal,
            pos: 7,
            length: 1,
        },
        Token {
            src: 0,
            kind: TokenKind::CloseParen,
            pos: 8,
            length: 1,
        },
    ];

    let mut parser = Parser::new();
    assert!(
        parser
            .parse(&tokens, &[], &LineIndex::from_bytes(&[]))
            .is_ok()
    );

    let json = parser.to_json();
    assert!(!json.is_empty());
}

#[test]
fn test_token_kind_conversion() {
    use std::convert::TryFrom;
    assert!(TokenKind::try_from(0).is_ok());
    assert!(TokenKind::try_from(16).is_ok());
    assert!(TokenKind::try_from(-1).is_err());
    assert!(TokenKind::try_from(17).is_err());
}
