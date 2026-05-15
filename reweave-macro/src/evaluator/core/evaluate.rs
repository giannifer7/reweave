use super::*;

impl Evaluator {
    pub fn evaluate(&mut self, node: &ASTNode) -> EvalResult<String> {
        let mut out = String::new();
        match node.kind {
            NodeKind::Text | NodeKind::Space | NodeKind::Ident => {
                let txt = self.node_text(node);
                out.push_str(&txt);
            }
            NodeKind::Var => {
                let var_name = self.node_text(node);
                let val = match self.state.get_variable_opt(&var_name) {
                    Some(v) => v,
                    None => return Err(EvalError::UndefinedVariable(var_name)),
                };
                out.push_str(&val);
            }
            NodeKind::Macro => {
                let name = self.node_text(node);
                let expansion = self.evaluate_macro_call(node, &name)?;
                out.push_str(&expansion);
            }
            NodeKind::Block | NodeKind::Param => {
                for child in &node.parts {
                    let s = self.evaluate(child)?;
                    out.push_str(&s);
                }
            }
            NodeKind::LineComment | NodeKind::BlockComment => {}
            _ => {
                for child in &node.parts {
                    let s = self.evaluate(child)?;
                    out.push_str(&s);
                }
            }
        }
        Ok(out)
    }
}
