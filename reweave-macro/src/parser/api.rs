use super::*;

impl Parser {
    /// Get a reference to a node by index
    pub fn get_node(&self, idx: usize) -> Option<&ParseNode> {
        self.nodes.get(idx)
    }

    /// Get a mutable reference to a node by index
    pub fn get_node_mut(&mut self, idx: usize) -> Option<&mut ParseNode> {
        self.nodes.get_mut(idx)
    }

    pub fn get_node_info(&self, idx: usize) -> Option<(&ParseNode, NodeKind)> {
        self.nodes.get(idx).map(|node| (node, node.kind))
    }

    #[cfg(test)]
    pub fn to_json(&self) -> String {
        self.nodes
            .iter()
            .map(|node| node.to_json())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get the root node's index (usually 0 if parse succeeded)
    pub fn get_root_index(&self) -> Option<usize> {
        if self.nodes.is_empty() {
            None
        } else {
            Some(0) // The root is always the first node
        }
    }

    /// Process AST including space stripping
    pub fn process_ast(&mut self, content: &[u8]) -> Result<ASTNode, String> {
        let root_idx = self
            .get_root_index()
            .ok_or_else(|| "Empty parse tree".to_string())?;

        crate::ast::strip_space_before_comments(content, self, root_idx)
            .map_err(|e| e.to_string())?;

        crate::ast::build_ast(self).map_err(|e| e.to_string())
    }

    /// Direct build without space stripping
    pub fn build_ast(&self) -> Result<ASTNode, String> {
        crate::ast::build_ast(self).map_err(|e| e.to_string())
    }

    /// Strip ending spaces from a node's token
    pub fn strip_ending_space(&mut self, content: &[u8], node_idx: usize) -> Result<(), String> {
        let node = self
            .get_node_mut(node_idx)
            .ok_or_else(|| format!("Node index {} not found", node_idx))?;

        let start = node.token.pos;
        let end = node.token.pos + node.token.length;
        if start >= content.len() {
            return Ok(());
        }

        let mut space_count = 0;
        for c in content[start..end.min(content.len())].iter().rev() {
            if *c == b'\n' || *c == b'\r' || *c == b' ' || *c == b'\t' {
                space_count += 1;
            } else {
                break;
            }
        }

        if space_count > 0 {
            node.token.length = node.token.length.saturating_sub(space_count);
            node.end_pos = node.token.pos + node.token.length;
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn add_node(&mut self, node: ParseNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }
}
