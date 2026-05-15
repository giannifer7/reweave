use crate::evaluator::{EvalConfig, Evaluator};
use crate::macro_api::process_string;

#[test]
fn test_export_at_global_scope_warns() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let _ = process_string("%def(foo, bar)%export(foo)", None, &mut eval).unwrap();
    let warnings = eval.take_warnings();
    assert!(
        warnings.iter().any(|w| w.contains("global scope")),
        "expected global-scope export warning, got: {warnings:?}"
    );
}

#[test]
fn test_export_inside_macro_does_not_warn() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let src = "%def(outer, %{\
                   %def(inner, x)\
                   %export(inner)\
               %})\
               %outer()";
    let _ = process_string(src, None, &mut eval).unwrap();
    let warnings = eval.take_warnings();
    assert!(
        !warnings.iter().any(|w| w.contains("global scope")),
        "unexpected global-scope warning inside macro: {warnings:?}"
    );
}

#[test]
fn test_if_no_args_warns() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let _ = process_string("%if()", None, &mut eval).unwrap();
    let warnings = eval.take_warnings();
    assert!(
        warnings.iter().any(|w| w.contains("%if")),
        "expected %if no-args warning, got: {warnings:?}"
    );
}

#[test]
fn test_take_warnings_drains() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let _ = process_string("%if()", None, &mut eval).unwrap();
    let first = eval.take_warnings();
    assert!(!first.is_empty());
    let second = eval.take_warnings();
    assert!(second.is_empty(), "take_warnings should drain the list");
}

#[test]
fn test_normal_if_does_not_warn() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let _ = process_string("%if(yes, ok)", None, &mut eval).unwrap();
    assert!(eval.take_warnings().is_empty());
}
