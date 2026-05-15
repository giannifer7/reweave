// crates/reweave-macro/src/lexer/mod.rs — generated from lexer.adoc
use crate::types::{LexerError, Token, TokenKind};
use memchr::{memchr, memmem};

#[cfg(test)]
mod tests;

fn is_identifier_start(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'_')
}

fn is_identifier_continue(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    /// Opening byte offset of the `%{` or `%name{`.
    Block(usize),
    /// Opening byte offset of the `%[` or `%name[`.
    Verbatim(usize),
    /// Opening byte offset of the `%name(`.
    Macro(usize),
    /// Comment state self-reports its own unclosed error.
    Comment,
}

/// What `handle_after_sigil` tells the caller to do.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SpecialAction {
    /// A new state was pushed; return `true` to keep the current state active.
    Push,
    /// A block was closed; return `false` to pop the current state.
    Pop,
    /// A token was emitted; continue the loop.
    Continue,
}

pub struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    src: u32,
    tokens: Vec<Token>,
    /// Configurable macro sigil (e.g. `%`, `@`, `§`).
    sigil: char,
    /// UTF-8 bytes of `sigil`, used by the byte-oriented scanner.
    sigil_bytes: Vec<u8>,
    state_stack: Vec<State>,
    pub errors: Vec<LexerError>,
    /// Precomputed `<sigil>/*` — checked in comment state.
    open_comment: Vec<u8>,
    /// Precomputed `<sigil>*/`.
    close_comment: Vec<u8>,
}

mod cursor;
mod emit;
mod run;
mod sigil;
mod states;

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str, sigil: char, src: u32) -> Self {
        let sigil_bytes = sigil.to_string().into_bytes();
        let mut open_comment = sigil_bytes.clone();
        open_comment.extend_from_slice(b"/*");
        let mut close_comment = sigil_bytes.clone();
        close_comment.extend_from_slice(b"*/");
        let mut lexer = Lexer {
            bytes: input.as_bytes(),
            pos: 0,
            src,
            tokens: Vec::new(),
            sigil,
            sigil_bytes,
            state_stack: Vec::new(),
            errors: Vec::new(),
            open_comment,
            close_comment,
        };
        lexer.state_stack.push(State::Block(0));
        lexer
    }

    pub fn lex(mut self) -> (Vec<Token>, Vec<LexerError>) {
        self.run();
        (self.tokens, self.errors)
    }
}
