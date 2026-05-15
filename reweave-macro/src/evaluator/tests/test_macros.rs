use crate::macro_api::process_string_defaults;

#[test]
fn test_simple_macro_definition() {
    let source = "%def(test_macro, simple text)\n%test_macro()";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap().trim(), "simple text");
}

#[test]
fn test_macro_with_parameters() {
    let source = "%def(greet, name, %{Hello, %(name)!%})\n%greet(World)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "Hello, World!"
    );
}

#[test]
fn test_param_substitution() {
    let source = "%def(test, param, wrap %(param) here)\n%test(value)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "wrap value here"
    );
}

#[test]
fn test_param_paren_precedence() {
    let source = "%def(test, p, text with %(p) here)\n%test(hello)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "text with hello here"
    );
}

#[test]
fn test_complex_param_substitution() {
    let source = "%def(format, text, %{\nbefore %(text),\nmiddle %(text))text,\nafter (%(text))\n%})\n%format(hello)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "before hello,\nmiddle hello)text,\nafter (hello)"
    );
}

#[test]
fn test_multiple_params_and_block() {
    let source =
        "%def(test, a, b, c, %{\n1:%(a)\n2:%(b)\n3:%(c)\n%})\n%test(first,\n second,\n third)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "1:first\n2:second\n3:third"
    );
}

#[test]
fn test_mixed_simple_and_block_bodies() {
    let source = "%def(simple, just text)\n%def(with_param, p,text with %(p))\n%def(complex, %{\ntext with,\nmultiple) lines\n%})\n%simple()\n%with_param(hello)\n%complex()";
    let result = process_string_defaults(source).unwrap();
    let result_str = std::str::from_utf8(&result).unwrap();
    assert!(result_str.contains("just text"));
    assert!(result_str.contains("text with hello"));
    assert!(result_str.contains("text with,\nmultiple) lines"));
}

#[test]
fn test_nested_macro_calls() {
    let source =
        "%def(inner, x, %{inner(%(x))%})\n%def(outer, y, %{outer(%inner(%(y)))%})\n%outer(test)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(
        std::str::from_utf8(&result).unwrap().trim(),
        "outer(inner(test))"
    );
}

#[test]
fn test_scope_isolation() {
    let source = "%def(m1, x, %(x))\n%def(m2, x, %m1(other_%(x)))\n%m2(value)";
    let result = process_string_defaults(source).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap().trim(), "other_value");
}
