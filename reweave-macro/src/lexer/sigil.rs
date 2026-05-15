use super::*;

impl<'a> Lexer<'a> {
    // ── Shared sigil-sequence handler ────────────────────────────────────
    //
    // Called after the sigil has been consumed.
    // `pct_start` is the byte offset of the sigil itself.

    pub(in crate::lexer) fn handle_after_sigil(&mut self, pct_start: usize) -> SpecialAction {
        match self.peek_byte() {
            Some(b'(') => {
                self.handle_var(pct_start);
                SpecialAction::Continue
            }
            Some(b'{') => {
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::BlockOpen);
                self.state_stack.push(State::Block(pct_start));
                SpecialAction::Push
            }
            Some(b'}') => {
                if self.state_stack.len() <= 1 {
                    self.error_at(pct_start, "Unmatched block close: no open block");
                }
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::BlockClose);
                SpecialAction::Pop
            }
            Some(b'[') => {
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimOpen);
                self.state_stack.push(State::Verbatim(pct_start));
                SpecialAction::Push
            }
            Some(b']') => {
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimClose);
                self.error_at(pct_start, "Unmatched verbatim close outside verbatim block");
                SpecialAction::Continue
            }
            Some(b'/') => {
                self.advance();
                match self.peek_byte() {
                    Some(b'/') => {
                        self.advance();
                        self.skip_line_comment();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::LineComment);
                    }
                    Some(b'*') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::CommentOpen);
                        self.state_stack.push(State::Comment);
                        return SpecialAction::Push;
                    }
                    _ => {
                        self.error_at(
                            pct_start,
                            &format!("Unexpected char after '{}/': expected // or /*", self.sigil),
                        );
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Text);
                    }
                }
                SpecialAction::Continue
            }
            Some(b'-') => {
                self.advance();
                match self.peek_byte() {
                    Some(b'-') => {
                        self.advance();
                        self.skip_line_comment();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::LineComment);
                    }
                    _ => {
                        self.error_at(
                            pct_start,
                            &format!("Unexpected char after '{}-': expected --", self.sigil),
                        );
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Text);
                    }
                }
                SpecialAction::Continue
            }
            Some(b'#') => {
                self.advance();
                self.skip_line_comment();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::LineComment);
                SpecialAction::Continue
            }
            Some(_) if self.starts_with_sigil() => {
                self.advance_sigil();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::Special);
                SpecialAction::Continue
            }
            Some(b) if is_identifier_start(b) => {
                let id_end = self.get_identifier_end(self.pos);
                self.pos = id_end;
                match self.peek_byte() {
                    Some(b'(') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Macro);
                        self.state_stack.push(State::Macro(pct_start));
                        SpecialAction::Push
                    }
                    Some(b'{') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::BlockOpen);
                        self.state_stack.push(State::Block(pct_start));
                        SpecialAction::Push
                    }
                    Some(b'}') => {
                        if self.state_stack.len() <= 1 {
                            self.error_at(pct_start, "Unmatched block close: no open block");
                        }
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::BlockClose);
                        SpecialAction::Pop
                    }
                    Some(b'[') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimOpen);
                        self.state_stack.push(State::Verbatim(pct_start));
                        SpecialAction::Push
                    }
                    Some(b']') => {
                        self.advance();
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::VerbatimClose);
                        self.error_at(pct_start, "Unmatched verbatim close outside verbatim block");
                        SpecialAction::Continue
                    }
                    _ => {
                        // %identifier not followed by ( { } — pass through as plain text.
                        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Text);
                        SpecialAction::Continue
                    }
                }
            }
            Some(_) => {
                // % followed by an unrecognized byte — emit just the % as Text with an error.
                // The unrecognized byte is left for the next iteration.
                self.error_at(
                    pct_start,
                    &format!("Unrecognized char after '{}'", self.sigil),
                );
                self.emit_token(pct_start, self.sigil_bytes.len(), TokenKind::Text);
                SpecialAction::Continue
            }
            None => {
                // % at EOF — emit as plain text.
                self.emit_token(pct_start, self.sigil_bytes.len(), TokenKind::Text);
                SpecialAction::Continue
            }
        }
    }

    /// Handle a `%(varname)` sequence. `pct_start` is the byte offset of the `%`.
    pub(in crate::lexer) fn handle_var(&mut self, pct_start: usize) {
        self.advance(); // consume '('
        let ident_start = self.pos;
        let ident_end = self.get_identifier_end(ident_start);
        if ident_end > ident_start {
            self.pos = ident_end;
            if self.peek_byte() == Some(b')') {
                self.advance();
                self.emit_token(pct_start, self.pos - pct_start, TokenKind::Var);
                return;
            }
            self.error_at(pct_start, "Var missing closing ')'");
        } else {
            self.error_at(
                pct_start,
                &format!("Var missing identifier after '{}('", self.sigil),
            );
        }
        self.emit_token(pct_start, self.pos - pct_start, TokenKind::Text);
    }

    // ── Comment state ─────────────────────────────────────────────────────

    pub(in crate::lexer) fn run_comment_state(&mut self) -> bool {
        let comment_text_start = self.pos;

        loop {
            // Jump to the next sigil — only it can start a delimiter.
            let rest = &self.bytes[self.pos..];
            let Some(i) = memmem::find(rest, &self.sigil_bytes) else {
                break; // EOF inside comment
            };
            self.pos += i;

            if self.starts_with_bytes(&self.open_comment) {
                if self.pos > comment_text_start {
                    self.emit_token(
                        comment_text_start,
                        self.pos - comment_text_start,
                        TokenKind::Text,
                    );
                }
                let delim_start = self.pos;
                self.pos += self.open_comment.len();
                self.emit_token(delim_start, self.open_comment.len(), TokenKind::CommentOpen);
                self.state_stack.push(State::Comment);
                return true;
            }
            if self.starts_with_bytes(&self.close_comment) {
                if self.pos > comment_text_start {
                    self.emit_token(
                        comment_text_start,
                        self.pos - comment_text_start,
                        TokenKind::Text,
                    );
                }
                let delim_start = self.pos;
                self.pos += self.close_comment.len();
                self.emit_token(
                    delim_start,
                    self.close_comment.len(),
                    TokenKind::CommentClose,
                );
                return false;
            }
            // Special char that isn't a comment delimiter — skip past it.
            self.pos += self.sigil_bytes.len();
        }

        // EOF: unclosed comment.
        if self.pos > comment_text_start {
            self.emit_token(
                comment_text_start,
                self.pos - comment_text_start,
                TokenKind::Text,
            );
        }
        self.error_at(comment_text_start, "Unclosed comment");
        false
    }
}
