use super::*;

impl Evaluator {
    pub fn do_include(&mut self, filename: &str) -> EvalResult<String> {
        let path = self.find_file(filename)?;

        if self.state.dependency_discovery_active {
            self.state.discovered_dependency_paths.push(path);
            return Ok("".into());
        }

        if self.state.open_includes.contains(&path) {
            return Err(EvalError::CircularInclude(path.display().to_string()));
        }
        self.state.open_includes.insert(path.clone());
        let result = (|| {
            let content = std::fs::read_to_string(&path)
                .map_err(|_| EvalError::IncludeNotFound(filename.into()))?;
            let ast = self.parse_string(&content, &path)?;
            self.evaluate(&ast)
        })();
        // Always remove the path, whether the include succeeded or failed,
        // so that a reused evaluator does not permanently block future includes.
        self.state.open_includes.remove(&path);
        result
    }

    /// Return (and clear) the list of paths recorded during dependency discovery.
    pub fn take_discovered_dependency_paths(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.state.discovered_dependency_paths)
    }

    pub(crate) fn set_dependency_discovery_active(&mut self, enabled: bool) {
        self.state.dependency_discovery_active = enabled;
    }
}
