// crates/reweave-macro/src/evaluator/tests/test_monty_eval.rs
use std::collections::HashMap;

use crate::evaluator::monty_eval::MontyEvaluator;

#[test]
fn test_monty_evaluate_basic_expression() {
    let eval = MontyEvaluator::new();
    let result = eval
        .evaluate(
            "str(int(x) * 2)",
            &["x".into()],
            &["21".into()],
            &HashMap::new(),
            Some("double"),
        )
        .unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_monty_evaluate_store_visible_but_declared_param_shadows() {
    let eval = MontyEvaluator::new();
    let mut store = HashMap::new();
    store.insert("prefix".to_string(), "item_".to_string());
    store.insert("name".to_string(), "store_value".to_string());

    let result = eval
        .evaluate(
            "prefix + name",
            &["name".into()],
            &["count".into()],
            &store,
            Some("tagged"),
        )
        .unwrap();
    assert_eq!(result, "item_count");
}

#[test]
fn test_monty_evaluate_runtime_error_contains_macro_name() {
    let eval = MontyEvaluator::new();
    let err = eval
        .evaluate(
            "1 / 0",
            &["x".into()],
            &["21".into()],
            &HashMap::new(),
            Some("broken"),
        )
        .unwrap_err();
    assert!(err.contains("pydef 'broken': runtime error"));
}

#[test]
fn test_monty_default_matches_new() {
    let eval = MontyEvaluator::default();
    let result = eval
        .evaluate("x", &["x".into()], &["ok".into()], &HashMap::new(), None)
        .unwrap();
    assert_eq!(result, "ok");
}

#[test]
fn test_monty_evaluate_formats_bool_and_none_results() {
    let eval = MontyEvaluator::new();
    let true_result = eval
        .evaluate("1 < 2", &[], &[], &HashMap::new(), Some("cmp"))
        .unwrap();
    let none_result = eval
        .evaluate("None", &[], &[], &HashMap::new(), Some("none"))
        .unwrap();
    assert_eq!(true_result, "true");
    assert_eq!(none_result, "");
}

#[test]
fn test_monty_evaluate_formats_float_and_list_results() {
    let eval = MontyEvaluator::new();
    let float_result = eval
        .evaluate("1.5", &[], &[], &HashMap::new(), Some("floaty"))
        .unwrap();
    let list_result = eval
        .evaluate(
            "[\"a\", \"b\", 3]",
            &[],
            &[],
            &HashMap::new(),
            Some("items"),
        )
        .unwrap();
    assert_eq!(float_result, "1.5");
    assert_eq!(list_result, "ab3");
}

#[test]
fn test_monty_formats_false_and_other_objects() {
    let eval = MontyEvaluator::new();
    let false_result = eval
        .evaluate("1 > 2", &[], &[], &HashMap::new(), Some("cmp"))
        .unwrap();
    let dict_result = eval
        .evaluate("{}", &[], &[], &HashMap::new(), Some("dict"))
        .unwrap();

    assert_eq!(false_result, "");
    assert!(dict_result.contains("Dict") || dict_result.contains("{}"));
}

#[test]
fn test_monty_evaluate_compile_error_contains_macro_name() {
    let eval = MontyEvaluator::new();
    let err = eval
        .evaluate("def broken(", &[], &[], &HashMap::new(), Some("broken"))
        .unwrap_err();
    assert!(err.contains("pydef 'broken': compile error"));
}
