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
