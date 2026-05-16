mod blocks_vars_realworld;
mod completion_boundaries;
mod core_tokens;
mod errors_comments;
mod smoke;
mod unicode_sigil;
mod verbatim;

use crate::lexer::{Lexer, State};
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

#[test]
fn lexer_low_level_helpers_cover_noop_paths() {
    let mut lexer = Lexer::new("abc", '%', 0);

    assert!(!lexer.advance_sigil());
    lexer.emit_token(0, 0, TokenKind::Text);
    assert!(lexer.tokens.is_empty());
}

#[test]
fn lexer_reports_unclosed_anonymous_block_and_macro_at_eof() {
    let (_, errors) = Lexer::new("%{", '%', 0).lex();
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unclosed anonymous block"))
    );

    let (_, errors) = Lexer::new("%name(", '%', 0).lex();
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unclosed macro argument list"))
    );
}

#[test]
fn lexer_reports_unmatched_tagged_block_close() {
    let (_, errors) = Lexer::new("%tag}", '%', 0).lex();

    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unmatched block close"))
    );
}

#[test]
fn lexer_direct_verbatim_state_reports_unmatched_close_edges() {
    let mut lexer = Lexer::new("%]", '%', 0);
    lexer.state_stack.clear();
    lexer.run_verbatim_state();
    assert!(
        lexer
            .errors
            .iter()
            .any(|err| err.message.contains("Unmatched verbatim close"))
    );

    let mut lexer = Lexer::new("%tag]", '%', 0);
    lexer.state_stack.clear();
    lexer.run_verbatim_state();
    assert!(
        lexer
            .errors
            .iter()
            .any(|err| err.message.contains("Unmatched verbatim close"))
    );
}

#[test]
fn lexer_unclosed_comment_emits_pending_text() {
    let (tokens, errors) = Lexer::new("%/* body", '%', 0).lex();

    assert!(tokens.iter().any(|token| token.kind == TokenKind::Text));
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unclosed comment"))
    );
}

#[test]
fn lexer_unclosed_comment_after_non_delimiter_sigil_emits_tail_text() {
    let (tokens, errors) = Lexer::new("%/* body %x", '%', 0).lex();

    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Text && token.length > 3)
    );
    assert!(
        errors
            .iter()
            .any(|err| err.message.contains("Unclosed comment"))
    );
}

#[test]
fn lexer_macro_state_returns_when_sigil_opens_nested_state() {
    let mut lexer = Lexer::new("%{", '%', 0);
    lexer.state_stack.clear();
    lexer.state_stack.push(State::Macro(0));

    assert!(lexer.run_macro_state());
}

#[test]
fn lexer_eof_with_comment_state_does_not_duplicate_comment_errors() {
    let mut lexer = Lexer::new("", '%', 0);
    lexer.state_stack.push(State::Comment);

    lexer.run();

    assert!(lexer.errors.is_empty());
    assert!(
        lexer
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::EOF)
    );
}

#[test]
fn lexer_macro_state_returns_when_sigil_pops_state() {
    let mut lexer = Lexer::new("%}", '%', 0);
    lexer.state_stack.clear();
    lexer.state_stack.push(State::Block(0));
    lexer.state_stack.push(State::Macro(0));

    assert!(!lexer.run_macro_state());
}

#[test]
fn lexer_macro_state_returns_when_stack_is_not_macro_after_token() {
    let mut lexer = Lexer::new("name", '%', 0);
    lexer.state_stack.clear();

    assert!(!lexer.run_macro_state());
}
