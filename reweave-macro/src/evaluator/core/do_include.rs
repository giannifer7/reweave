use super::*;

impl Evaluator {
    pub fn do_include(&mut self, filename: &str) -> EvalResult<String> {
        let path = self.find_file(filename)?;

        if self.state.open_includes.contains(&path) {
            return Err(EvalError::CircularInclude(None, path.display().to_string()));
        }
        self.state.open_includes.insert(path.clone());
        let result = (|| {
            let content = std::fs::read_to_string(&path)
                .map_err(|_| EvalError::IncludeNotFound(None, filename.into()))?;
            let ast = self.parse_string(&content, &path)?;
            self.evaluate(&ast)
        })();
        // Always remove the path, whether the include succeeded or failed,
        // so that a reused evaluator does not permanently block future includes.
        self.state.open_includes.remove(&path);
        result
    }
}
