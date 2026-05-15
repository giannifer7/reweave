use super::*;

impl<'a> Lexer<'a> {
    // ── Main driver ───────────────────────────────────────────────────────

    pub fn run(&mut self) {
        loop {
            // EOF is driven by input exhaustion, not by stack state.
            if self.pos >= self.bytes.len() {
                // Collect before borrowing &mut self for error_at.
                let unclosed: Vec<(String, usize)> = self
                    .state_stack
                    .get(1..)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|s| match s {
                        State::Block(p) => {
                            let tag = self.block_tag_at(*p);
                            let msg = if tag.is_empty() {
                                "Unclosed anonymous block '%{'".to_string()
                            } else {
                                format!("Unclosed block '%{}{{'", tag)
                            };
                            Some((msg, *p))
                        }
                        State::Verbatim(p) => {
                            let tag = self.block_tag_at(*p);
                            let msg = if tag.is_empty() {
                                "Unclosed anonymous block '%['".to_string()
                            } else {
                                format!("Unclosed block '%{}['", tag)
                            };
                            Some((msg, *p))
                        }
                        State::Macro(p) => Some(("Unclosed macro argument list".to_string(), *p)),
                        State::Comment => None, // self-reported by run_comment_state
                    })
                    .collect();
                for (msg, pos) in unclosed {
                    self.error_at(pos, &msg);
                }
                self.emit_token(self.pos, 0, TokenKind::EOF);
                return;
            }
            let state = match self.state_stack.last().copied() {
                Some(s) => s,
                None => {
                    // Stack underflow before EOF — push/pop bug.
                    self.error_at(self.pos, "internal error: state stack underflow");
                    self.emit_token(self.pos, 0, TokenKind::EOF);
                    return;
                }
            };
            let keep_state = match state {
                State::Block(_) => self.run_block_state(),
                State::Verbatim(_) => self.run_verbatim_state(),
                State::Macro(_) => self.run_macro_state(),
                State::Comment => self.run_comment_state(),
            };
            if !keep_state {
                self.state_stack.pop();
            }
        }
    }
}
