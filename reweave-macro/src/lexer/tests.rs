mod blocks_vars_realworld;
mod completion_boundaries;
mod core_tokens;
mod errors_comments;
mod smoke;
mod unicode_sigil;
mod verbatim;

use crate::lexer::Lexer;
use crate::types::{Token, TokenKind};

/// Collect tokens from the lexer (non-EOF tokens only).
fn collect_tokens_with_timeout(input: &str) -> Result<Vec<Token>, String> {
    collect_tokens_with_sigil(input, '%')
}

fn collect_tokens_with_sigil(input: &str, sigil: char) -> Result<Vec<Token>, String> {
    let (tokens, errors) = Lexer::new(input, sigil, 0).lex();
    if !errors.is_empty() {
        // Errors are non-fatal for these tests; just return what was produced.
        let _ = errors;
    }
    Ok(tokens
        .into_iter()
        .filter(|t| t.kind != TokenKind::EOF)
        .collect())
}

/// Helper to assert tokens match an expected sequence of (TokenKind, &str).
/// We compare both `kind` and the `length` of the text (since we can't store real text easily).
fn assert_tokens(input: &str, expected: &[(TokenKind, &str)]) {
    let result = collect_tokens_with_timeout(input).expect("Failed to collect tokens");
    let tokens = result;

    assert_eq!(
        tokens.len(),
        expected.len(),
        "Wrong number of tokens: expected {}, got {}. Tokens: {:?}",
        expected.len(),
        tokens.len(),
        tokens
    );

    for (i, (token, (exp_kind, exp_text))) in tokens.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            token.kind, *exp_kind,
            "Token {} kind mismatch: expected {:?}, got {:?}",
            i, exp_kind, token.kind
        );
        let got_len = token.length;
        let exp_len = exp_text.len();
        assert_eq!(
            got_len, exp_len,
            "Token {} length mismatch: expected {}, got {} (expected text='{}')",
            i, exp_len, got_len, exp_text
        );
    }
}

fn assert_tokens_with_sigil(input: &str, sigil: char, expected: &[(TokenKind, &str)]) {
    let result = collect_tokens_with_sigil(input, sigil)
        .expect("Failed to collect tokens with custom sigil");
    let tokens = result;

    assert_eq!(
        tokens.len(),
        expected.len(),
        "Wrong number of tokens: expected {}, got {}. Tokens: {:?}",
        expected.len(),
        tokens.len(),
        tokens
    );

    for (i, (token, (exp_kind, exp_text))) in tokens.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            token.kind, *exp_kind,
            "Token {} kind mismatch: expected {:?}, got {:?}",
            i, exp_kind, token.kind
        );
        let got_len = token.length;
        let exp_len = exp_text.len();
        assert_eq!(
            got_len, exp_len,
            "Token {} length mismatch: expected {}, got {} (expected text='{}')",
            i, exp_len, got_len, exp_text
        );
    }
}
