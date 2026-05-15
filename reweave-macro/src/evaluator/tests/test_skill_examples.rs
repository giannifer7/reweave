// crates/reweave-macro/src/evaluator/tests/test_skill_examples.rs
//
// Tests that verify the exact examples shown in SKILL.md.

use crate::evaluator::EvalError;
use crate::macro_api::process_string_defaults;

/// SKILL.md — positional params, multi-line call, leading space stripped, %# comments stripped.
#[test]
fn test_tag_positional_space_stripped() {
    let input = "%def(tag, name, value, %{<%(name)>%(value)</%(name)>%})\n\
                 %tag( div,         %# element name — leading space stripped\n\
                       Hello world) %# value        — leading space stripped";
    let result = process_string_defaults(input).unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(
        s.contains("<div>Hello world</div>"),
        "expected '<div>Hello world</div>' in output, got: {s:?}"
    );
}

/// SKILL.md — %{ %} wrapping preserves the leading space inside the block.
#[test]
fn test_tag_block_preserves_leading_space() {
    let input = "%def(tag, name, value, %{<%(name)>%(value)</%(name)>%})\n\
                 %tag(%{ div%}, %{ Hello world%})";
    let result = process_string_defaults(input).unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(
        s.contains("< div> Hello world</ div>"),
        "expected '< div> Hello world</ div>' in output, got: {s:?}"
    );
}

/// SKILL.md — named parameters on multiple lines with interspersed whitespace.
#[test]
fn test_http_endpoint_named_params() {
    let input = "%def(http_endpoint, method, path, handler, %{\n\
                 %(method) %(path) \u{2192} %(handler)\n\
                 %})\n\
                 \n\
                 %http_endpoint(\n\
                     method  = GET,\n\
                     path    = /api/users,\n\
                     handler = list_users)";
    let result = process_string_defaults(input).unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(
        s.contains("GET /api/users \u{2192} list_users"),
        "expected 'GET /api/users \u{2192} list_users' in output, got: {s:?}"
    );
}

/// Too few arguments are a hard error in the strict language.
#[test]
fn test_too_few_args_is_error() {
    let err = process_string_defaults(
        "%def(greet, name, msg, Hello %(name)%(msg)!)\n\
         %greet(Alice)",
    )
    .unwrap_err();
    assert!(err.to_string().contains("Unbound parameter"));
}

/// Too many positional arguments is now an error (Phase 1 diagnostic).
#[test]
fn test_too_many_args_is_error() {
    let result = process_string_defaults(
        "%def(greet, name, Hello %(name)!)\n\
         %greet(Alice, Bob, Charlie)",
    );
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "expected InvalidUsage for extra positional args, got: {result:?}"
    );
}

/// Macro-local variable lookup is current-frame-only; outer variables do not leak.
#[test]
fn test_outer_variable_does_not_leak_into_macro_body() {
    let err = process_string_defaults(
        "%set(greeting, Hi)\n\
         %def(greet, name, %(greeting) %(name)!)\n\
         %set(greeting, Bye)\n\
         %greet(Alice)",
    )
    .unwrap_err();
    assert!(err.to_string().contains("Undefined variable"));
}

/// Named params are matched by name; any order among named args is valid.
#[test]
fn test_named_params_any_order() {
    let result = process_string_defaults(
        "%def(http_endpoint, method, path, handler, \
              %(method) %(path) %(handler))\n\
         %http_endpoint(\n\
             handler = list_users,\n\
             method  = GET,\n\
             path    = /api/users)",
    )
    .unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(
        s.contains("GET /api/users list_users"),
        "named params in reverse order should still bind by name, got: {s:?}"
    );
}

/// A trailing comma after the last named argument is accepted.
#[test]
fn test_named_params_allow_trailing_comma() {
    let result = process_string_defaults(
        "%def(http_endpoint, method, path, handler, \
              %(method) %(path) %(handler))\n\
         %http_endpoint(\n\
             handler = list_users,\n\
             method = GET,\n\
             path = /api/users,\n\
         )",
    )
    .unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(
        s.contains("GET /api/users list_users"),
        "expected trailing comma to be ignored, got: {s:?}"
    );
}

/// Positional before named: the first param is positional, the rest named.
#[test]
fn test_positional_before_named() {
    let result = process_string_defaults(
        "%def(f, a, b, c, %(a)-%(b)-%(c))\n\
         %f(X, c = Z, b = Y)",
    )
    .unwrap();
    let s = std::str::from_utf8(&result).unwrap();
    assert!(s.contains("X-Y-Z"), "expected 'X-Y-Z', got: {s:?}");
}

/// Positional after named → error.
#[test]
fn test_positional_after_named_is_error() {
    let result = process_string_defaults(
        "%def(f, a, b, %(a)-%(b))\n\
         %f(a = X, Y)",
    );
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "expected InvalidUsage for positional-after-named, got: {result:?}"
    );
}

/// Binding the same param positionally and by name → error.
#[test]
fn test_double_bind_is_error() {
    let result = process_string_defaults(
        "%def(f, a, b, %(a)-%(b))\n\
         %f(X, a = Y)",
    );
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "expected InvalidUsage for double-bind, got: {result:?}"
    );
}

/// Unknown named param is an error (helps catch typos).
#[test]
fn test_unknown_named_param_is_error() {
    let result = process_string_defaults(
        "%def(greet, name, Hello %(name)!)\n\
         %greet(name = Alice, typo = oops)",
    );
    assert!(
        matches!(result, Err(EvalError::InvalidUsage(_))),
        "expected InvalidUsage for unknown named arg, got: {result:?}"
    );
}
