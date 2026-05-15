use super::*;

impl Evaluator {
    pub fn define_macro(
        &mut self,
        mac: crate::evaluator::state::MacroDefinition,
    ) -> EvalResult<()> {
        self.state.define_macro(mac)
    }

    pub fn redefine_macro(
        &mut self,
        mac: crate::evaluator::state::MacroDefinition,
    ) -> EvalResult<()> {
        self.state.redefine_macro(mac)
    }

    pub fn get_macro(&self, name: &str) -> Option<crate::evaluator::state::MacroDefinition> {
        self.state.get_macro(name)
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.state.set_variable(name, value);
    }

    pub fn evaluate_with_temporary_variables(
        &mut self,
        bindings: &[(String, String)],
        node: &ASTNode,
    ) -> EvalResult<String> {
        let mut seen = HashSet::new();
        let mut saved = Vec::new();
        {
            let frame = self.state.current_scope_mut();
            for (name, value) in bindings {
                if !seen.insert(name.clone()) {
                    frame.variables.insert(
                        name.clone(),
                        TrackedValue {
                            value: value.clone(),
                            spans: vec![],
                        },
                    );
                    continue;
                }
                saved.push((name.clone(), frame.variables.get(name).cloned()));
                frame.variables.insert(
                    name.clone(),
                    TrackedValue {
                        value: value.clone(),
                        spans: vec![],
                    },
                );
            }
        }

        let result = self.evaluate(node);

        let frame = self.state.current_scope_mut();
        for (name, old_value) in saved.into_iter().rev() {
            if let Some(old_value) = old_value {
                frame.variables.insert(name, old_value);
            } else {
                frame.variables.remove(&name);
            }
        }

        result
    }

    pub fn record_var_def(&mut self, var_name: String, src: u32, pos: u32, length: u32) {
        self.state
            .var_defs
            .push(crate::evaluator::state::VarDefRaw {
                var_name,
                src,
                pos,
                length,
            });
    }

    pub fn record_macro_def(&mut self, macro_name: String, src: u32, pos: u32, length: u32) {
        self.state
            .macro_defs
            .push(crate::evaluator::state::MacroDefRaw {
                macro_name,
                src,
                pos,
                length,
            });
    }

    pub fn drain_var_defs(&mut self) -> Vec<crate::evaluator::state::VarDefRaw> {
        self.state.drain_var_defs()
    }

    pub fn drain_macro_defs(&mut self) -> Vec<crate::evaluator::state::MacroDefRaw> {
        self.state.drain_macro_defs()
    }

    pub fn push_warning(&mut self, msg: String) {
        self.state.push_warning(msg);
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        self.state.drain_warnings()
    }
}
