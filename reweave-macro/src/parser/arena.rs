use super::*;

impl Parser {
    pub fn new() -> Self {
        Parser {
            nodes: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub(in crate::parser) fn create_node(
        &mut self,
        kind: NodeKind,
        src: u32,
        token: Token,
    ) -> usize {
        let node = ParseNode {
            kind,
            src,
            token,
            end_pos: 0,
            parts: Vec::new(),
        };
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    pub(in crate::parser) fn add_child(&mut self, parent_idx: usize, child_idx: usize) {
        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            parent.parts.push(child_idx);
        }
    }

    pub(in crate::parser) fn create_add_node(
        &mut self,
        kind: NodeKind,
        src: u32,
        token: Token,
    ) -> usize {
        let new_idx = self.create_node(kind, src, token);
        // Attach it to the node on top of the stack
        if let Some(&(_, parent_idx)) = self.stack.last() {
            self.add_child(parent_idx, new_idx);
        }
        new_idx
    }

    /// Convenience: `create_add_node` + push a new stack frame in one call.
    pub(in crate::parser) fn push_node(
        &mut self,
        state: ParserState,
        kind: NodeKind,
        src: u32,
        token: Token,
    ) -> usize {
        let idx = self.create_add_node(kind, src, token);
        self.stack.push((state, idx));
        idx
    }

    /// Set `end_pos` on the node currently at the top of the stack.
    /// Must be called *before* the corresponding `stack.pop()`.
    /// Returns `Err` rather than panicking so callers can propagate cleanly.
    pub(in crate::parser) fn close_top(&mut self, end: usize) -> Result<(), ParserError> {
        let (_, node_idx) = *self
            .stack
            .last()
            .ok_or_else(|| ParserError::Parse("internal error: close_top on empty stack".into()))?;
        self.nodes
            .get_mut(node_idx)
            .ok_or_else(|| {
                ParserError::Parse(format!(
                    "internal error: close_top node {node_idx} not in arena"
                ))
            })?
            .end_pos = end;
        Ok(())
    }

    /// Close all open nodes and clear the stack.  Called on both error and
    /// normal termination paths to keep the tree structurally consistent.
    pub(in crate::parser) fn unwind_stack(&mut self, end: usize) {
        while let Some(&(_, node_idx)) = self.stack.last() {
            debug_assert!(
                node_idx < self.nodes.len(),
                "unwind_stack: node_idx {node_idx} OOB"
            );
            self.nodes[node_idx].end_pos = end;
            self.stack.pop();
        }
    }
    /// Extract the tag sub-span from a `BlockOpen` or `BlockClose` token.
    /// For `%{` / `%}` (length 2) the tag is empty (tag_len == 0).
    /// For `%foo{` / `%foo}` (length > 2) the tag is bytes
    /// [pos+special_len .. pos+length-1].
    pub(in crate::parser) fn block_tag(token: &Token, content: &[u8]) -> (usize, usize) {
        let special_len = content
            .get(token.pos..)
            .and_then(|tail| std::str::from_utf8(tail).ok())
            .and_then(|s| s.chars().next())
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        let tag_len = token.length.saturating_sub(special_len + 1);
        (token.pos + special_len, tag_len)
    }
}
