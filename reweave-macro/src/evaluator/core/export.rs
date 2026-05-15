use super::*;

impl Evaluator {
    pub fn export(&mut self, name: &str) {
        let stack_len = self.state.scope_stack.len();
        if stack_len <= 1 {
            self.state.push_warning(format!(
                "%export('{name}') at global scope has no effect (no parent frame to export into)"
            ));
            return;
        }
        let parent_index = stack_len - 2;

        if let Some(val) = self
            .state
            .scope_stack
            .last()
            .unwrap()
            .variables
            .get(name)
            .cloned()
        {
            self.state
                .scope_stack
                .get_mut(parent_index)
                .unwrap()
                .variables
                .insert(name.to_string(), val);
        }

        if let Some(mac) = self
            .state
            .scope_stack
            .last()
            .unwrap()
            .macros
            .get(name)
            .cloned()
        {
            // Plain upward copy — no automatic free-variable freezing.
            // Use %alias(new, src, k=v) for explicit capture.
            self.state
                .scope_stack
                .get_mut(parent_index)
                .unwrap()
                .macros
                .insert(name.to_string(), mac);
        }
    }
}
