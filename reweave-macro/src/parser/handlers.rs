use super::*;

impl Parser {
    /// Handle a token when the top of the stack is a `Block`.
    /// `tag_pos`/`tag_len` come directly from the caller's pattern match —
    /// no second stack lookup needed.
    /// Returns `Ok(true)` if the token was consumed (caller should `continue`).
    pub(in crate::parser) fn handle_block(
        &mut self,
        token: Token,
        ctx: &ParseContext,
        tag_pos: usize,
        tag_len: usize,
        delim: BlockDelim,
    ) -> Result<bool, ParserError> {
        let expected_close = match delim {
            BlockDelim::Curly => TokenKind::BlockClose,
            BlockDelim::Square => TokenKind::VerbatimClose,
        };
        if token.kind != expected_close {
            return Ok(false);
        }
        let close_tag = Self::block_tag(&token, ctx.content);
        let open_tag = (tag_pos, tag_len);
        if !ctx.tags_match(open_tag, close_tag) {
            let (ol, oc) = ctx.line_col(tag_pos);
            let (cl, cc) = ctx.line_col(token.pos);
            let (open_ch, close_ch) = block_delim_chars(delim);
            let open_label = block_tag_label(ctx.tag_str(open_tag.0, open_tag.1), open_ch);
            let close_label = block_tag_label(ctx.tag_str(close_tag.0, close_tag.1), close_ch);
            let err = ParserError::Parse(format!(
                "{cl}:{cc}: block tag mismatch: '{close_label}' does not close '{open_label}' (opened at {ol}:{oc})",
            ));
            self.unwind_stack(token.end());
            return Err(err);
        }
        self.close_top(token.end())?;
        self.stack.pop();
        Ok(true)
    }
    /// Handle a token when the top of the stack is a `Param`.
    /// Returns `Ok(true)` if the token was consumed (caller should `continue`).
    pub(in crate::parser) fn handle_param(&mut self, token: Token) -> Result<bool, ParserError> {
        match token.kind {
            TokenKind::Comma => {
                // Close current Param, open next Param.
                self.close_top(token.pos)?;
                self.stack.pop();
                self.push_node(ParserState::Param, NodeKind::Param, token.src, token);
                Ok(true)
            }
            TokenKind::CloseParen => {
                // Close Param, then Macro (which must be directly below).
                self.close_top(token.end())?;
                self.stack.pop();
                match self.stack.last() {
                    Some((ParserState::Macro, _)) => {
                        self.close_top(token.end())?;
                        self.stack.pop();
                    }
                    _ => {
                        self.unwind_stack(token.end());
                        return Err(ParserError::Parse(
                            "internal error: expected Macro below Param".into(),
                        ));
                    }
                }
                Ok(true)
            }
            TokenKind::Ident => {
                self.create_add_node(NodeKind::Ident, token.src, token);
                Ok(true)
            }
            TokenKind::Space => {
                self.create_add_node(NodeKind::Space, token.src, token);
                Ok(true)
            }
            TokenKind::Equal => {
                self.create_add_node(NodeKind::Equal, token.src, token);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    /// Handle a token when the top of the stack is a `Comment`.
    /// Comment state always consumes every token, so no bool return is needed.
    pub(in crate::parser) fn handle_comment(&mut self, token: Token) -> Result<(), ParserError> {
        match token.kind {
            TokenKind::CommentClose => {
                self.close_top(token.end())?;
                self.stack.pop();
            }
            TokenKind::CommentOpen => {
                // Nested comment.
                self.push_node(
                    ParserState::Comment,
                    NodeKind::BlockComment,
                    token.src,
                    token,
                );
            }
            _ => {} // Inside a comment everything is opaque.
        }
        Ok(())
    }
}
