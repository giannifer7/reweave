use super::*;

impl Evaluator {
    pub fn evaluate_macro_call(&mut self, node: &ASTNode, name: &str) -> EvalResult<String> {
        if let Some(bf) = self.builtins.get(name) {
            return bf(self, node);
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

        // Evaluate ALL arguments in CALLER scope, before pushing the callee frame.
        // This means %(var) in an argument resolves against the caller's bindings.
        // Effectful builtins like %set are rejected in argument position.
        self.validate_argument_side_effects(&param_nodes, &mac.name)?;
        let binding_plan = self.plan_macro_bindings(&mac, &param_nodes)?;
        if let Some(param_name) = binding_plan.unbound.first() {
            return Err(EvalError::UnboundParameter {
                macro_name: mac.name.clone(),
                param_name: (*param_name).to_string(),
            });
        }

        let mut positional_vals: Vec<String> = Vec::new();
        for binding in &binding_plan.positional {
            positional_vals.push(self.evaluate(binding.param_node)?);
        }
        let mut named_vals: Vec<String> = Vec::new();
        for binding in &binding_plan.named {
            named_vals.push(self.evaluate(binding.param_node)?);
        }

        self.state.push_scope();

        // frozen_args: free variables pre-bound by %alias(…, k=v) overrides
        for (var, frozen_val) in mac.frozen_args.iter() {
            self.state.set_variable(var, frozen_val);
        }

        for (binding, val) in binding_plan.positional.iter().zip(positional_vals.iter()) {
            self.state.set_variable(binding.param_name, val);
        }
        for (binding, val) in binding_plan.named.iter().zip(named_vals.iter()) {
            self.state.set_variable(&binding.arg_name, val);
        }
        for param_name in &binding_plan.unbound {
            self.state.set_variable(param_name, "");
        }

        self.state.call_depth += 1;
        let result = self.evaluate(&mac.body);
        self.state.call_depth -= 1;
        let mut result = result?;

        match mac.script_kind {
            ScriptKind::None => {}
            ScriptKind::Python => {
                // Pass only the explicitly declared parameters to the Python script;
                // the store is injected as additional variables (params shadow store).
                let args: Vec<String> = mac
                    .params
                    .iter()
                    .map(|p| self.state.get_variable(p))
                    .collect();
                result = self
                    .monty_evaluator
                    .evaluate(&result, &mac.params, &args, &self.py_store, Some(&mac.name))
                    .map_err(EvalError::Runtime)?;
            }
        }

        self.state.pop_scope();

        Ok(result)
    }
}
