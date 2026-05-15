use super::*;

impl<'a> Lexer<'a> {
    // ── Block state ───────────────────────────────────────────────────────

    pub(in crate::lexer) fn run_block_state(&mut self) -> bool {
        loop {
            let rest = &self.bytes[self.pos..];
            let text_len = match memmem::find(rest, &self.sigil_bytes) {
                Some(i) => i,
                None => {
                    if !rest.is_empty() {
                        self.emit_token(self.pos, rest.len(), TokenKind::Text);
                        self.pos = self.bytes.len();
                    }
                    return false;
                }
            };
            if text_len > 0 {
                self.emit_token(self.pos, text_len, TokenKind::Text);
                self.pos += text_len;
            }
            let pct_start = self.pos;
            self.advance_sigil(); // consume the sigil
            match self.handle_after_sigil(pct_start) {
                SpecialAction::Push => return true,
                SpecialAction::Pop => return false,
                SpecialAction::Continue => {}
            }
        }
    }

    // ── Verbatim state ────────────────────────────────────────────────────

    pub(in crate::lexer) fn run_verbatim_state(&mut self) -> bool {
        loop {
            let rest = &self.bytes[self.pos..];
            let text_len = match memmem::find(rest, &self.sigil_bytes) {
                Some(i) => i,
                None => {
                    if !rest.is_empty() {
                        self.emit_token(self.pos, rest.len(), TokenKind::Text);
                        self.pos = self.bytes.len();
                    }
                    return false;
                }
            };
            if text_len > 0 {
                self.emit_token(self.pos, text_len, TokenKind::Text);
                self.pos += text_len;
            }

            let pct_start = self.pos;
            self.advance_sigil();

            if self.peek_byte() == Some(b'[') {
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimOpen);
                self.state_stack.push(State::Verbatim(pct_start));
                return true;
            }

            if self.peek_byte() == Some(b']') {
                if self.state_stack.len() <= 1 {
                    self.error_at(pct_start, "Unmatched verbatim close: no open block");
                }
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimClose);
                return false;
            }

            if self.peek_byte().is_some_and(is_identifier_start) {
                let id_end = self.get_identifier_end(self.pos);
                self.pos = id_end;
                match self.peek_byte() {
                    Some(b'[') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimOpen);
                        self.state_stack.push(State::Verbatim(pct_start));
                        return true;
                    }
                    Some(b']') => {
                        if self.state_stack.len() <= 1 {
                            self.error_at(pct_start, "Unmatched verbatim close: no open block");
                        }
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimClose);
                        return false;
                    }
                    _ => {
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Text);
                        continue;
                    }
                }
            }

            self.emit_token(pct_start, self.sigil_bytes.len(), TokenKind::Text);
        }
    }

    // ── Macro arg state ───────────────────────────────────────────────────

    pub(in crate::lexer) fn run_macro_state(&mut self) -> bool {
        while let Some(b) = self.peek_byte() {
            if b == b')' {
                let start = self.pos;
                self.advance();
                self.emit_token(start, 1, TokenKind::CloseParen);
                return false;
            } else if b == b',' {
                let start = self.pos;
                self.advance();
                self.emit_token(start, 1, TokenKind::Comma);
            } else if b == b'=' {
                let start = self.pos;
                self.advance();
                self.emit_token(start, 1, TokenKind::Equal);
            } else if is_whitespace(b) {
                let ws_start = self.pos;
                while self.peek_byte().is_some_and(is_whitespace) {
                    self.advance();
                }
                self.emit_token(ws_start, self.pos - ws_start, TokenKind::Space);
            } else if self.starts_with_sigil() {
                let pct_start = self.pos;
                self.advance_sigil();
                match self.handle_after_sigil(pct_start) {
                    SpecialAction::Push => return true,
                    SpecialAction::Pop => return false,
                    SpecialAction::Continue => {}
                }
            } else if is_identifier_start(b) {
                let start = self.pos;
                let end = self.get_identifier_end(start);
                self.pos = end;
                self.emit_token(start, end - start, TokenKind::Ident);
            } else {
                let start = self.pos;
                while let Some(b2) = self.peek_byte() {
                    if is_whitespace(b2)
                        || matches!(b2, b')' | b',' | b'=')
                        || self.bytes[self.pos..].starts_with(&self.sigil_bytes)
                    {
                        break;
                    }
                    self.advance();
                }
                self.emit_token(start, self.pos - start, TokenKind::Text);
            }

            if !matches!(self.state_stack.last(), Some(State::Macro(_))) {
                return false;
            }
        }
        // EOF without closing ')'.
        if let Some(&State::Macro(open_pos)) = self.state_stack.last() {
            self.error_at(open_pos, "Unclosed macro argument list");
        }
        false
    }
}
