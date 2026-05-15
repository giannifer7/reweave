use super::*;

impl Evaluator {
    /// Like `evaluate`, but writes to an `EvalOutput` sink so that span
    /// information is available to the caller.
    pub fn evaluate_to(&mut self, node: &ASTNode, out: &mut dyn EvalOutput) -> EvalResult<()> {
        self.evaluate_to_with_context(node, out, None)
    }

    /// Internal evaluation method that accepts an optional `context_span` prefix.
    /// This is used to thread `MacroBody` attribution down the evaluation tree.
    pub(super) fn evaluate_to_with_context(
        &mut self,
        node: &ASTNode,
        out: &mut dyn EvalOutput,
        context_span: Option<&SourceSpan>,
    ) -> EvalResult<()> {
        match node.kind {
            NodeKind::Text | NodeKind::Space | NodeKind::Ident => {
                let txt = self.node_text(node);
                // Build the base span: use the token's own src/pos/length for
                // exact position, but inherit `kind` from context (e.g. MacroBody).
                let base_span = if let Some(ctx) = context_span {
                    let mut s = self.span_of(node);
                    s.kind = ctx.kind.clone();
                    s
                } else {
                    self.span_of(node)
                };
                // Multi-line text tokens (common in macro bodies where the lexer
                // groups all literal text between two macro calls) have a single
                // `pos` pointing to the start of the token.  Split at newlines
                // and advance `pos` by byte offset within the token so that every
                // line segment resolves to its true source line/col.
                let base_pos = base_span.pos;
                let mut offset = 0usize;
                for segment in txt.split_inclusive('\n') {
                    let mut seg_span = base_span.clone();
                    seg_span.pos = base_pos + offset;
                    seg_span.length = segment.len();
                    out.push_str(segment, seg_span);
                    offset += segment.len();
                }
            }
            NodeKind::Var => {
                let var_name = self.node_text(node);
                if let Some(tracked) = self.state.get_tracked_variable(&var_name) {
                    if tracked.spans.is_empty() {
                        if out.is_tracing() && !tracked.value.is_empty() {
                            // Emit a coarse VarBinding span so the tracer can identify the
                            // variable and its `set_locations`.  Position points to the
                            // %(var_name) token — i.e. the usage site, not the definition.
                            let value = tracked.value.clone();
                            let mut base_span = self.span_of(node);
                            base_span.kind = SpanKind::VarBinding {
                                var_name: var_name.clone(),
                            };
                            let base_pos = base_span.pos;
                            let mut offset = 0;
                            for segment in value.split_inclusive('\n') {
                                let mut seg_span = base_span.clone();
                                seg_span.pos = base_pos + offset;
                                seg_span.length = segment.len();
                                out.push_str(segment, seg_span);
                                offset += segment.len();
                            }
                        } else {
                            // Untracked: computed/script result or unbound parameter.
                            out.push_untracked(&tracked.value);
                        }
                    } else {
                        // Replay each attributed range in order.
                        // Multiple ranges = full per-token threading through argument evaluation.
                        // Single range covering [0, len] = coarse call-site span (fast path).
                        for range in &tracked.spans {
                            out.push_str(
                                &tracked.value[range.start..range.end],
                                range.span.clone(),
                            );
                        }
                    }
                } else {
                    return Err(EvalError::UndefinedVariable(var_name));
                }
            }
            NodeKind::Macro => {
                let name = self.node_text(node);
                self.evaluate_macro_call_to(node, &name, out)?;
            }
            NodeKind::Block | NodeKind::Param => {
                for child in &node.parts {
                    self.evaluate_to_with_context(child, out, context_span)?;
                }
            }
            NodeKind::LineComment | NodeKind::BlockComment => {}
            _ => {
                for child in &node.parts {
                    self.evaluate_to_with_context(child, out, context_span)?;
                }
            }
        }
        Ok(())
    }
}
