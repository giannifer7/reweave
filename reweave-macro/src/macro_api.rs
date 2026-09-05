use crate::evaluator::{EvalConfig, EvalError, Evaluator};
use std::path::{Path, PathBuf};

/// Expand macros in `source`, attributing the input to `real_path` (used for
/// error locations and `%include` resolution).
pub fn process_string(
    source: &str,
    real_path: Option<&Path>,
    evaluator: &mut Evaluator,
) -> Result<Vec<u8>, EvalError> {
    let path_for_parsing = match real_path {
        Some(rp) => rp.to_path_buf(),
        None => PathBuf::from(format!("<string-{}>", evaluator.num_source_files())),
    };
    let ast = evaluator.parse_string(source, &path_for_parsing)?;
    evaluator.validate_ast_semantics(&ast)?;
    if let Some(rp) = real_path {
        evaluator.set_current_file(rp.to_path_buf());
    }
    let output_string = evaluator.evaluate(&ast)?;
    Ok(output_string.into_bytes())
}

pub fn process_string_defaults(source: &str) -> Result<Vec<u8>, EvalError> {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    process_string(source, None, &mut evaluator)
}
