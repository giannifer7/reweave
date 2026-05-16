use super::*;

impl Evaluator {
    // ---- Tracked evaluation (EvalOutput) ------------------------------------

    /// Build a `SourceSpan` from the token of an AST node, defaulting to Literal.
    pub(super) fn span_of(&self, node: &ASTNode) -> SourceSpan {
        SourceSpan {
            src: node.token.src,
            pos: node.token.pos,
            length: node.token.length,
            kind: SpanKind::Literal,
        }
    }

    /// Evaluate `node` into a `(String, Vec<SpanRange>)` for argument threading.
    /// Called only on the tracing path (`out.is_tracing() == true`).
    pub(super) fn evaluate_arg_to_traced(
        &mut self,
        node: &ASTNode,
    ) -> EvalResult<(String, Vec<SpanRange>)> {
        let mut arg_out = PreciseTracingOutput::new();
        self.evaluate_to(node, &mut arg_out)?;
        Ok(arg_out.into_parts())
    }

    /// Re-tag `raw_spans` to `MacroArg { macro_name, param_name }`.
    /// If `raw_spans` is empty but `val` is non-empty, creates a single coarse span
    /// from `param_node` so the tracer can still identify the parameter.
    pub(super) fn tag_as_macro_arg(
        &self,
        raw_spans: Vec<SpanRange>,
        val: &str,
        param_node: &ASTNode,
        macro_name: &str,
        param_name: &str,
    ) -> Vec<SpanRange> {
        let kind = SpanKind::MacroArg {
            macro_name: macro_name.to_string(),
            param_name: param_name.to_string(),
        };
        if raw_spans.is_empty() && !val.is_empty() {
            let mut s = self.span_of(param_node);
            s.kind = kind;
            vec![SpanRange {
                start: 0,
                end: val.len(),
                span: s,
            }]
        } else {
            raw_spans
                .into_iter()
                .map(|mut sr| {
                    sr.span.kind = kind.clone();
                    sr
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::types::{ASTNode, NodeKind, Token, TokenKind};

    #[test]
    fn tag_as_macro_arg_builds_coarse_span_for_untracked_values() {
        let mut eval = Evaluator::new(EvalConfig::default());
        let src = eval.add_source_bytes(b"value".to_vec(), PathBuf::from("arg.txt"));
        let param = ASTNode {
            kind: NodeKind::Param,
            src,
            token: Token {
                src,
                kind: TokenKind::Text,
                pos: 0,
                length: 5,
            },
            end_pos: 5,
            name: None,
            parts: vec![],
        };

        let spans = eval.tag_as_macro_arg(Vec::new(), "value", &param, "wrap", "x");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 5);
        assert!(matches!(
            spans[0].span.kind,
            SpanKind::MacroArg {
                ref macro_name,
                ref param_name
            } if macro_name == "wrap" && param_name == "x"
        ));
    }
}
