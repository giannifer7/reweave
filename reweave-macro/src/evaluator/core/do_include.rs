use super::*;

impl Evaluator {
    pub fn do_include(&mut self, filename: &str) -> EvalResult<String> {
        let path = self.find_file(filename)?;
        self.state.included_paths.push(path.clone());

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

    /// Like `do_include`, but idempotent per run: a repeat `%import` of the same resolved path
    /// is a silent no-op rather than re-parsing and re-evaluating the file (which would redefine
    /// its macros and fail as a duplicate binding - see `imported_paths`' own doc comment).
    pub fn do_import(&mut self, filename: &str) -> EvalResult<String> {
        let path = self.find_file(filename)?;
        if self.state.imported_paths.contains(&path) {
            self.state.included_paths.push(path);
            return Ok(String::new());
        }
        self.state.imported_paths.insert(path);
        self.do_include(filename)
    }

    /// Return (and clear) the include paths resolved so far.
    pub fn drain_included_paths(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.state.included_paths)
    }
}
