use crate::evaluator::EvalError;
use crate::macro_api::process_string_defaults;

#[test]
fn test_eq_equal_strings() {
    assert_eq!(process_string_defaults("%eq(hello, hello)").unwrap(), b"1");
}

#[test]
fn test_eq_unequal_strings() {
    assert_eq!(process_string_defaults("%eq(hello, world)").unwrap(), b"");
}

#[test]
fn test_eq_empty_strings() {
    // Use %{%} (empty raw body) to pass genuine empty-string args
    assert_eq!(process_string_defaults("%eq(%{%}, %{%})").unwrap(), b"1");
}

#[test]
fn test_neq_unequal() {
    assert_eq!(process_string_defaults("%neq(a, b)").unwrap(), b"1");
}

#[test]
fn test_neq_equal() {
    assert_eq!(process_string_defaults("%neq(a, a)").unwrap(), b"");
}

#[test]
fn test_not_empty_is_true() {
    assert_eq!(process_string_defaults("%not()").unwrap(), b"1");
}

#[test]
fn test_not_nonempty_is_false() {
    assert_eq!(process_string_defaults("%not(x)").unwrap(), b"");
}

#[test]
fn test_not_whitespace_only_arg_is_stripped_by_parser() {
    // The parser strips whitespace-only arguments before evaluation,
    // so %not(   ) receives an empty string and returns "1".
    assert_eq!(process_string_defaults("%not(   )").unwrap(), b"1");
}

#[test]
fn test_eq_used_in_if() {
    let r = process_string_defaults(
        "%def(x, %{a%})\
         %if(%eq(%x(), a), yes, no)",
    )
    .unwrap();
    assert_eq!(std::str::from_utf8(&r).unwrap().trim(), "yes");
}

#[test]
fn test_not_used_in_if() {
    // %not() with no args == "1" (truthy) → if picks the true branch
    let r = process_string_defaults("%if(%not(), absent, present)").unwrap();
    assert_eq!(std::str::from_utf8(&r).unwrap().trim(), "absent");
}

#[test]
fn test_eq_wrong_arity() {
    assert!(matches!(
        process_string_defaults("%eq(a)"),
        Err(EvalError::InvalidUsage(_))
    ));
}

#[test]
fn test_neq_wrong_arity() {
    assert!(matches!(
        process_string_defaults("%neq(a)"),
        Err(EvalError::InvalidUsage(_))
    ));
}

#[test]
fn test_not_wrong_arity() {
    assert!(matches!(
        process_string_defaults("%not(a, b)"),
        Err(EvalError::InvalidUsage(_))
    ));
}

#[test]
fn test_eq_neq_are_inverse() {
    // For any pair (a, b), eq and neq return opposite canonical booleans
    let eq_same = process_string_defaults("%eq(x, x)").unwrap();
    let neq_same = process_string_defaults("%neq(x, x)").unwrap();
    assert_eq!(eq_same, b"1");
    assert_eq!(neq_same, b"");

    let eq_diff = process_string_defaults("%eq(x, y)").unwrap();
    let neq_diff = process_string_defaults("%neq(x, y)").unwrap();
    assert_eq!(eq_diff, b"");
    assert_eq!(neq_diff, b"1");
}

#[test]
fn test_builtin_name_guard_rejects_eq_redefinition() {
    // Attempting to redefine a builtin should fail
    assert!(matches!(
        process_string_defaults("%def(eq, a, b, %(a))"),
        Err(EvalError::InvalidUsage(_))
    ));
    assert!(matches!(
        process_string_defaults("%def(not, x, %(x))"),
        Err(EvalError::InvalidUsage(_))
    ));
    assert!(matches!(
        process_string_defaults("%redef(eq, a, b, %(a))"),
        Err(EvalError::InvalidUsage(_))
    ));
}
