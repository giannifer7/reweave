use super::*;

impl Evaluator {
    /// Like `evaluate_macro_call`, but writes to an `EvalOutput` sink.
    pub fn evaluate_macro_call_to(
        &mut self,
        node: &ASTNode,
        name: &str,
        out: &mut dyn EvalOutput,
    ) -> EvalResult<()> {
        // Builtins: delegate to plain path, then emit with a Computed span so
        // the tracer attributes the call-site line/byte position.
        // Builtins that return "" (set, def, include, …) produce no output.
        if self.builtins.contains_key(name) {
            let result = self.evaluate_macro_call(node, name)?;
            if !result.is_empty() {
                let mut span = self.span_of(node);
                span.kind = SpanKind::Computed;
                out.push_str(&result, span);
            }
            return Ok(());
        }

        if self.state.call_depth >= self.state.config.recursion_limit {
            return Err(EvalError::Runtime(format!(
                "maximum recursion depth ({}) exceeded in macro '{}'",
                self.state.config.recursion_limit, name
            )));
        }

        let mac = match self.state.get_macro(name) {
            Some(m) => m,
            None => return Err(EvalError::UndefinedMacro(name.into())),
        };

        let param_nodes = Self::macro_param_nodes(node);

        // Plan bindings and evaluate arguments in CALLER scope, before push_scope.
        self.validate_argument_side_effects(&param_nodes, &mac.name)?;
        let binding_plan = self.plan_macro_bindings(&mac, &param_nodes)?;
        if let Some(param_name) = binding_plan.unbound.first() {
            return Err(EvalError::UnboundParameter {
                macro_name: mac.name.clone(),
                param_name: (*param_name).to_string(),
            });
        }

        // Pre-evaluate all args: (param_name, val, tagged_spans, coarse_span).
        // tagged_spans is non-empty only when out.is_tracing().
        let mut positional_pre: Vec<(String, String, Vec<SpanRange>, SourceSpan)> = Vec::new();
        for binding in &binding_plan.positional {
            if out.is_tracing() {
                let (val, raw_spans) = self.evaluate_arg_to_traced(binding.param_node)?;
                let tagged = self.tag_as_macro_arg(
                    raw_spans,
                    &val,
                    binding.param_node,
                    name,
                    binding.param_name,
                );
                positional_pre.push((
                    binding.param_name.to_string(),
                    val,
                    tagged,
                    self.span_of(binding.param_node),
                ));
            } else {
                let val = self.evaluate(binding.param_node)?;
                let mut span = self.span_of(binding.param_node);
                span.kind = SpanKind::MacroArg {
                    macro_name: name.to_string(),
                    param_name: binding.param_name.to_string(),
                };
                positional_pre.push((binding.param_name.to_string(), val, vec![], span));
            }
        }

        let mut named_pre: Vec<(String, String, Vec<SpanRange>, SourceSpan)> = Vec::new();
        for binding in &binding_plan.named {
            if out.is_tracing() {
                let (val, raw_spans) = self.evaluate_arg_to_traced(binding.param_node)?;
                let tagged = self.tag_as_macro_arg(
                    raw_spans,
                    &val,
                    binding.param_node,
                    name,
                    &binding.arg_name,
                );
                named_pre.push((
                    binding.arg_name.clone(),
                    val,
                    tagged,
                    self.span_of(binding.param_node),
                ));
            } else {
                let val = self.evaluate(binding.param_node)?;
                let mut span = self.span_of(binding.param_node);
                span.kind = SpanKind::MacroArg {
                    macro_name: name.to_string(),
                    param_name: binding.arg_name.clone(),
                };
                named_pre.push((binding.arg_name.clone(), val, vec![], span));
            }
        }

        let unbound_names: Vec<String> =
            binding_plan.unbound.iter().map(|s| s.to_string()).collect();

        // NOW push the callee frame
        self.state.push_scope();

        // frozen_args: pre-bound free variables from %alias(…, k=v) overrides
        for (var, frozen_val) in mac.frozen_args.iter() {
            let mut span = self.span_of(node);
            span.kind = SpanKind::VarBinding {
                var_name: var.clone(),
            };
            self.state.set_tracked_variable(var, frozen_val, Some(span));
        }

        // Bind pre-evaluated positional args
        for (param_name, val, tagged, coarse_span) in positional_pre {
            if out.is_tracing() {
                self.state.set_traced_variable(&param_name, val, tagged);
            } else {
                self.state
                    .set_tracked_variable(&param_name, &val, Some(coarse_span));
            }
        }

        // Bind pre-evaluated named args
        for (param_name, val, tagged, coarse_span) in named_pre {
            if out.is_tracing() {
                self.state.set_traced_variable(&param_name, val, tagged);
            } else {
                self.state
                    .set_tracked_variable(&param_name, &val, Some(coarse_span));
            }
        }

        for param_name in &unbound_names {
            self.state.set_variable(param_name, "");
        }

        self.state.call_depth += 1;

        // Pass down a MacroBody context span so all literal text in the body is
        // correctly attributed as coming from a macro body expansion.
        let mut body_span = self.span_of(&mac.body);
        body_span.kind = SpanKind::MacroBody {
            macro_name: mac.name.clone(),
        };

        let body_result = self.evaluate_to_with_context(&mac.body, out, Some(&body_span));
        self.state.call_depth -= 1;
        body_result?;

        self.state.pop_scope();

        Ok(())
    }
}
