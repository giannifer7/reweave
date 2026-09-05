use super::*;
use crate::line_index::LineIndex;

impl Evaluator {
    /// Resolve an AST node to its 1-indexed `file:line:col` position, if the
    /// node's source file is known.
    pub fn node_location(&self, node: &ASTNode) -> Option<SourceLocation> {
        let src = node.src;
        let bytes = self.state.source_manager.get_source(src)?;
        let (line, col) = LineIndex::from_bytes(bytes).line_col(node.token.pos);
        let file = self
            .state
            .source_manager
            .source_files()
            .get(src as usize)?
            .display()
            .to_string();
        Some(SourceLocation { file, line, col })
    }
}
