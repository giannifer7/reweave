use crate::evaluator::{EvalConfig, Evaluator};
use std::path::Path;

/// Create an EvalConfig whose include path lives inside `temp_dir`.
/// Callers must keep the `TempDir` alive for the test duration;
/// on drop, Rust cleans up the directory automatically.
pub fn config_in_temp_dir(temp_dir: &Path) -> EvalConfig {
    EvalConfig {
        include_paths: vec![temp_dir.to_path_buf()],
        ..Default::default()
    }
}

/// Convenience wrapper: evaluator whose working files stay inside `temp_dir`.
pub fn evaluator_in_temp_dir(temp_dir: &Path) -> Evaluator {
    Evaluator::new(config_in_temp_dir(temp_dir))
}
