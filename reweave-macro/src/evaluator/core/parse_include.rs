use super::*;

impl Evaluator {
    pub fn parse_string(&mut self, text: &str, path: &PathBuf) -> Result<ASTNode, EvalError> {
        let src = match fs::metadata(path) {
            Ok(md) if md.is_file() => self.add_source_if_not_present(path.clone())?,
            _ => self.add_source_bytes(text.as_bytes().to_vec(), path.clone()),
        };

        let result =
            crate::evaluator::lexer_parser::lex_parse_content(text, self.state.config.sigil, src);
        result.map_err(EvalError::ParseError)
    }

    pub(super) fn find_file(&self, filename: &str) -> EvalResult<PathBuf> {
        let p = Path::new(filename);
        if p.is_absolute() && p.exists() {
            return Ok(p.to_path_buf());
        }
        for inc in &self.state.config.include_paths {
            let candidate = inc.join(filename);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(EvalError::IncludeNotFound(filename.into()))
    }
}
