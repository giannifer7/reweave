use super::*;

impl Parser {
    /// Main parse function.
    /// `content` is the raw source bytes — used for block-tag comparison and diagnostics.
    /// `line_index` is borrowed from the caller, who may have built it already for
    /// lexer-error formatting, so the O(n) newline scan happens at most once per source.
    pub fn parse(
        &mut self,
        tokens: &[Token],
        content: &[u8],
        line_index: &LineIndex,
    ) -> Result<(), ParserError> {
        self.nodes.clear();
        self.stack.clear();

        if tokens.is_empty() {
            return Ok(());
        }

        let ctx = ParseContext::new(content, line_index);

        // Root is a synthetic Block: it has no source token.
        let root_idx = self.create_node(
            NodeKind::Block,
            tokens[0].src,
            Token::synthetic(tokens[0].src, 0),
        );
        self.stack.push((
            ParserState::Block {
                tag_pos: 0,
                tag_len: 0,
                delim: BlockDelim::Curly,
            },
            root_idx,
        ));

        for token in tokens {
            let token = *token;

            // EOF is a structural sentinel, not an AST node.
            if token.kind == TokenKind::EOF {
                break;
            }

            let consumed = match self.stack.last().map(|&(st, _)| st) {
                Some(ParserState::Block {
                    tag_pos,
                    tag_len,
                    delim,
                }) => self.handle_block(token, &ctx, tag_pos, tag_len, delim)?,
                Some(ParserState::Param) => self.handle_param(token)?,
                Some(ParserState::Macro) => {
                    // Macro is always below Param; receiving a token here is an
                    // internal invariant violation.
                    self.unwind_stack(token.end());
                    return Err(ParserError::Parse(
                        "internal error: token received in Macro state (Param expected on top)"
                            .into(),
                    ));
                }
                Some(ParserState::Comment) => {
                    self.handle_comment(token)?;
                    true
                }
                None => {
                    return Err(ParserError::Parse(
                        "internal error: empty parser stack".into(),
                    ));
                }
            };

            if consumed {
                continue;
            }

            // Tokens that open new structure or become leaf nodes.
            match token.kind {
                TokenKind::BlockOpen => {
                    let (tag_pos, tag_len) = Self::block_tag(&token, ctx.content);
                    self.push_node(
                        ParserState::Block {
                            tag_pos,
                            tag_len,
                            delim: BlockDelim::Curly,
                        },
                        NodeKind::Block,
                        token.src,
                        token,
                    );
                }
                TokenKind::VerbatimOpen => {
                    let (tag_pos, tag_len) = Self::block_tag(&token, ctx.content);
                    self.push_node(
                        ParserState::Block {
                            tag_pos,
                            tag_len,
                            delim: BlockDelim::Square,
                        },
                        NodeKind::Block,
                        token.src,
                        token,
                    );
                }
                TokenKind::Macro => {
                    // Push Macro node, then an initial synthetic Param node on top.
                    self.push_node(ParserState::Macro, NodeKind::Macro, token.src, token);
                    self.push_node(
                        ParserState::Param,
                        NodeKind::Param,
                        token.src,
                        Token::synthetic(token.src, token.pos),
                    );
                }
                TokenKind::CommentOpen => {
                    self.push_node(
                        ParserState::Comment,
                        NodeKind::BlockComment,
                        token.src,
                        token,
                    );
                }
                TokenKind::Var => {
                    self.create_add_node(NodeKind::Var, token.src, token);
                }
                TokenKind::LineComment => {
                    self.create_add_node(NodeKind::LineComment, token.src, token);
                }
                _ => {
                    // Structural tokens must be handled above; anything else
                    // (Text, Space, Special, stray punctuation) falls through as Text.
                    debug_assert!(
                        !matches!(
                            token.kind,
                            TokenKind::BlockOpen
                                | TokenKind::VerbatimOpen
                                | TokenKind::CommentOpen
                                | TokenKind::Macro
                                | TokenKind::Var
                                | TokenKind::LineComment
                                | TokenKind::EOF
                        ),
                        "unexpected structural token in Text fallback: {:?}",
                        token.kind
                    );
                    self.create_add_node(NodeKind::Text, token.src, token);
                }
            }
        }

        let end = tokens.last().map(|t| t.end()).unwrap_or(0);

        // Report unclosed non-root structures (stack[0] is always the root block).
        if self.stack.len() > 1 {
            let err = match self.stack.last().map(|&(st, idx)| (st, idx)) {
                Some((
                    ParserState::Block {
                        tag_pos,
                        tag_len,
                        delim,
                    },
                    _,
                )) => {
                    let (open_ch, _) = block_delim_chars(delim);
                    let label = block_tag_label(ctx.tag_str(tag_pos, tag_len), open_ch);
                    let (line, col) = ctx.line_col(tag_pos);
                    ParserError::Parse(format!("{line}:{col}: unclosed block '{label}'"))
                }
                Some((ParserState::Macro, idx)) | Some((ParserState::Param, idx)) => {
                    let pos = self
                        .nodes
                        .get(idx)
                        .expect("node index from stack must exist in arena")
                        .token
                        .pos;
                    let (line, col) = ctx.line_col(pos);
                    ParserError::Parse(format!("{line}:{col}: unclosed macro argument list"))
                }
                Some((ParserState::Comment, idx)) => {
                    let pos = self
                        .nodes
                        .get(idx)
                        .expect("node index from stack must exist in arena")
                        .token
                        .pos;
                    let (line, col) = ctx.line_col(pos);
                    ParserError::Parse(format!("{line}:{col}: unclosed block comment '%/*'"))
                }
                None => unreachable!(),
            };
            self.unwind_stack(end);
            return Err(err);
        }

        // Normal termination: close the root block.
        self.close_top(end)?;
        self.stack.pop();

        Ok(())
    }
}
