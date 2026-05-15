use super::*;

impl Evaluator {
    pub fn add_source_if_not_present(&mut self, file_path: PathBuf) -> Result<u32, std::io::Error> {
        self.state
            .source_manager
            .add_source_if_not_present(file_path)
    }

    pub fn add_source_bytes(&mut self, content: Vec<u8>, path: PathBuf) -> u32 {
        self.state.source_manager.add_source_bytes(content, path)
    }

    pub fn set_current_file(&mut self, file: PathBuf) {
        self.state.current_file = file;
    }

    pub fn get_current_file_path(&self) -> PathBuf {
        self.state.current_file.clone()
    }

    pub fn source_files(&self) -> &[PathBuf] {
        self.state.source_manager.source_files()
    }

    pub fn get_sigil(&self) -> Vec<u8> {
        self.state.get_sigil()
    }

    pub fn allow_env(&self) -> bool {
        self.state.config.allow_env
    }

    pub fn env_prefix(&self) -> Option<&str> {
        self.state.config.env_prefix.as_deref()
    }

    pub fn num_source_files(&self) -> usize {
        self.state.source_manager.num_sources()
    }
}
