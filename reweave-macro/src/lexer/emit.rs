use super::*;

impl<'a> Lexer<'a> {
    // ── Emission ──────────────────────────────────────────────────────────

    pub(in crate::lexer) fn emit_token(&mut self, pos: usize, length: usize, kind: TokenKind) {
        if length == 0 && kind != TokenKind::EOF {
            return;
        }
        self.tokens.push(Token {
            kind,
            src: self.src,
            pos,
            length,
        });
    }

    pub(in crate::lexer) fn error_at(&mut self, pos: usize, message: &str) {
        self.errors.push(LexerError {
            pos,
            message: message.to_string(),
        });
    }
}
