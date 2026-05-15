use super::*;

impl Evaluator {
    pub fn extract_name_value(&self, name_token: &Token) -> String {
        if let Some(source) = self.state.source_manager.get_source(name_token.src) {
            let start = name_token.pos;
            let end = name_token.pos + name_token.length;

            // Bounds checking
            if end > source.len() || start > source.len() {
                eprintln!(
                    "extract_name_value: out of range - start: {}, end: {}, source len: {}",
                    start,
                    end,
                    source.len()
                );
                return "".into();
            }

            // Since we know it's an Identifier, we can extract directly
            String::from_utf8_lossy(&source[start..end]).to_string()
        } else {
            eprintln!("extract_name_value: invalid src index");
            "".into()
        }
    }
}
