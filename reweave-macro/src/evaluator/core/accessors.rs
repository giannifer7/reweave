use super::*;

impl Evaluator {
    pub(super) fn macro_param_nodes(node: &ASTNode) -> Vec<&ASTNode> {
        node.parts
            .iter()
            .filter(|p| p.kind == NodeKind::Param)
            .collect()
    }

    fn validate_param_order(param_nodes: &[&ASTNode], macro_name: &str) -> EvalResult<usize> {
        let mut seen_named = false;
        for param_node in param_nodes {
            if param_node.name.is_some() {
                seen_named = true;
            } else if seen_named {
                return Err(EvalError::InvalidUsage(format!(
                    "macro '{}': positional argument follows named argument",
                    macro_name
                )));
            }
        }
        Ok(param_nodes.iter().take_while(|n| n.name.is_none()).count())
    }

    fn validate_named_arg_binding(
        declared: &HashSet<&str>,
        assigned: &HashSet<String>,
        arg_name: &str,
        macro_name: &str,
    ) -> EvalResult<()> {
        if !declared.contains(arg_name) {
            return Err(EvalError::InvalidUsage(format!(
                "macro '{}': unknown named argument '{arg_name}'",
                macro_name
            )));
        }
        if assigned.contains(arg_name) {
            return Err(EvalError::InvalidUsage(format!(
                "macro '{}': parameter '{arg_name}' bound both positionally and by name",
                macro_name
            )));
        }
        Ok(())
    }

    fn arg_contains_builtin_call(&self, node: &ASTNode, builtin_name: &str) -> bool {
        if node.kind == NodeKind::Macro && self.node_text(node) == builtin_name {
            return true;
        }

        node.parts
            .iter()
            .any(|child| self.arg_contains_builtin_call(child, builtin_name))
    }

    pub(super) fn validate_argument_side_effects(
        &self,
        param_nodes: &[&ASTNode],
        macro_name: &str,
    ) -> EvalResult<()> {
        for param_node in param_nodes {
            if self.arg_contains_builtin_call(param_node, "set") {
                return Err(EvalError::InvalidUsage(format!(
                    "macro '{macro_name}': %set is not allowed in argument position"
                )));
            }
        }
        Ok(())
    }

    pub fn validate_ast_semantics(&self, root: &ASTNode) -> EvalResult<()> {
        let _ = root;
        Ok(())
    }

    pub(super) fn plan_macro_bindings<'a>(
        &self,
        mac: &'a MacroDefinition,
        param_nodes: &[&'a ASTNode],
    ) -> EvalResult<BindingPlan<'a>> {
        let declared: HashSet<&str> = mac.params.iter().map(String::as_str).collect();
        let positional_count = Self::validate_param_order(param_nodes, &mac.name)?;
        let mut assigned: HashSet<String> = HashSet::new();
        let mut positional = Vec::new();
        let mut named = Vec::new();

        if positional_count > mac.params.len() {
            return Err(EvalError::InvalidUsage(format!(
                "macro '{}': {} positional argument(s) given, but only {} parameter(s) declared",
                mac.name,
                positional_count,
                mac.params.len()
            )));
        }

        for (i, param_node) in param_nodes[..positional_count].iter().enumerate() {
            let param_name = &mac.params[i];
            positional.push(PositionalBinding {
                param_name,
                param_node,
            });
            assigned.insert(param_name.clone());
        }

        for param_node in &param_nodes[positional_count..] {
            let arg_name = self.extract_name_value(param_node.name.as_ref().unwrap());
            Self::validate_named_arg_binding(&declared, &assigned, &arg_name, &mac.name)?;
            named.push(NamedBinding {
                arg_name: arg_name.clone(),
                param_node,
            });
            assigned.insert(arg_name);
        }

        let unbound = mac
            .params
            .iter()
            .filter_map(|param_name| {
                (!assigned.contains(param_name)).then_some(param_name.as_str())
            })
            .collect();

        Ok(BindingPlan {
            positional,
            named,
            unbound,
        })
    }

    pub fn new(config: EvalConfig) -> Self {
        Evaluator {
            state: EvaluatorState::new(config),
            builtins: default_builtins(),
            monty_evaluator: MontyEvaluator::new(),
            py_store: HashMap::new(),
        }
    }

    /// Access the underlying SourceManager (useful for mapping output spans back to lines/columns).
    pub fn sources(&self) -> &crate::evaluator::state::SourceManager {
        &self.state.source_manager
    }
}
