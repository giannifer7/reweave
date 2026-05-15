// crates/reweave-macro/src/evaluator/tests/test_env.rs
use crate::evaluator::{EvalConfig, EvalError, Evaluator};
use crate::macro_api::process_string;

#[test]
fn test_env_is_disabled_by_default() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let err = process_string("%env(PATH)", None, &mut eval).unwrap_err();
    assert!(matches!(err, EvalError::InvalidUsage(_)));
    assert!(err.to_string().contains("--allow-env"));
}

#[test]
fn test_env_reads_variable_when_enabled() {
    let mut eval = Evaluator::new(EvalConfig {
        allow_env: true,
        ..EvalConfig::default()
    });
    let result = process_string("%env(PATH)", None, &mut eval).unwrap();
    assert!(!String::from_utf8(result).unwrap().is_empty());
}

#[test]
fn test_env_with_no_args_returns_empty_when_enabled() {
    let mut eval = Evaluator::new(EvalConfig {
        allow_env: true,
        ..EvalConfig::default()
    });
    let result = process_string("%env()", None, &mut eval).unwrap();
    assert_eq!(String::from_utf8(result).unwrap(), "");
}

#[test]
fn test_env_prefix_is_applied_when_configured() {
    let key = "WBM_PREFIXED_ENV_TEST";
    unsafe {
        std::env::set_var(key, "prefixed-value");
    }

    let mut eval = Evaluator::new(EvalConfig {
        allow_env: true,
        env_prefix: Some("WBM_".into()),
        ..EvalConfig::default()
    });
    let result = process_string("%env(PREFIXED_ENV_TEST)", None, &mut eval).unwrap();
    assert_eq!(String::from_utf8(result).unwrap(), "prefixed-value");
}

#[test]
fn test_eval_requires_macro_name() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let err = process_string("%eval()", None, &mut eval).unwrap_err();
    assert!(matches!(err, EvalError::InvalidUsage(_)));
    assert!(err.to_string().contains("eval requires macroName"));
}

#[test]
fn test_eval_with_whitespace_only_arg_is_still_missing_name() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let err = process_string("%eval( )", None, &mut eval).unwrap_err();
    assert!(matches!(err, EvalError::InvalidUsage(_)));
    assert!(err.to_string().contains("eval requires macroName"));
}
