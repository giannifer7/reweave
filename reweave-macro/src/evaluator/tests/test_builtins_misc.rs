// crates/reweave-macro/src/evaluator/tests/test_builtins_misc.rs
use crate::evaluator::{EvalConfig, EvalError, Evaluator};
use crate::macro_api::process_string;
use tempfile::TempDir;

fn eval_default(src: &str) -> Result<String, EvalError> {
    let mut eval = Evaluator::new(EvalConfig::default());
    process_string(src, None, &mut eval).map(|b| String::from_utf8(b).unwrap())
}

#[test]
fn test_include_and_import_empty_filename_return_empty() {
    assert_eq!(eval_default("%include()").unwrap(), "");
    assert_eq!(eval_default("%import()").unwrap(), "");
}

#[test]
fn test_convert_case_wrong_arity_reports_error() {
    let err = eval_default("%convert_case(one)").unwrap_err();
    assert!(matches!(err, EvalError::InvalidUsage(_)));
    assert!(err.to_string().contains("convert_case: exactly 2 args"));
}

#[test]
fn test_convert_case_empty_input_returns_empty() {
    let result = eval_default("%convert_case(, snake)").unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_single_arg_case_builtins_empty_input_return_empty() {
    assert_eq!(eval_default("%capitalize()").unwrap(), "");
    assert_eq!(eval_default("%decapitalize()").unwrap(), "");
    assert_eq!(eval_default("%to_snake_case()").unwrap(), "");
}

#[test]
fn test_store_getters_require_key() {
    let py_err = eval_default("%pyget()").unwrap_err();
    assert!(py_err.to_string().contains("pyget: requires a key"));
}

#[test]
fn test_store_setters_require_two_args() {
    let err = eval_default("%pyset(one)").unwrap_err();
    assert!(matches!(err, EvalError::InvalidUsage(_)));
}

#[test]
fn test_discover_includes_in_file_records_import_target() {
    let temp = TempDir::new().unwrap();
    let include_path = temp.path().join("inc.txt");
    std::fs::write(&include_path, "%def(x, y)").unwrap();
    let main_path = temp.path().join("main.txt");
    std::fs::write(&main_path, "%import(inc.txt)").unwrap();

    let mut eval = Evaluator::new(EvalConfig {
        include_paths: vec![temp.path().to_path_buf()],
        ..EvalConfig::default()
    });

    let result = crate::macro_api::discover_includes_in_file(&main_path, &mut eval).unwrap();
    assert_eq!(result, vec![include_path]);
}
