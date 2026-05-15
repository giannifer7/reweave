use super::*;

impl Evaluator {
    pub fn node_text(&self, node: &ASTNode) -> String {
        if let Some(source) = self.state.source_manager.get_source(node.token.src) {
            let start = node.token.pos;
            let end = node.token.pos + node.token.length;
            if end > source.len() || start > source.len() {
                eprintln!(
                    "node_text: out of range - start: {}, end: {}, source len: {}",
                    start,
                    end,
                    source.len()
                );
                return "".into();
            }

            let special_len = std::str::from_utf8(&source[start..])
                .ok()
                .and_then(|s| s.chars().next())
                .map(|c| c.len_utf8())
                .unwrap_or(1);

            let slice = match node.token.kind {
                TokenKind::BlockOpen | TokenKind::BlockClose | TokenKind::Macro => {
                    if end > start + special_len + 1 {
                        &source[(start + special_len)..(end - 1)]
                    } else {
                        &source[start..end]
                    }
                }
                TokenKind::Var => {
                    if end > start + special_len + 2 {
                        &source[(start + special_len + 1)..(end - 1)]
                    } else {
                        &source[start..end]
                    }
                }
                TokenKind::Special => {
                    if end > start + special_len {
                        &source[start..(end - 1)]
                    } else {
                        &source[start..end]
                    }
                }
                _ => &source[start..end],
            };
            String::from_utf8_lossy(slice).to_string()
        } else {
            eprintln!("node_text: invalid src index");
            "".into()
        }
    }
}
