use crate::macro_api::process_string_defaults;

#[test]
fn test_builtin_capitalize() {
    let result = process_string_defaults(r#"%capitalize(hello)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello");
}

#[test]
fn test_builtin_decapitalize() {
    let result = process_string_defaults(r#"%decapitalize(HELLO)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hELLO");
}

#[test]
fn test_builtin_convert_case() {
    let result = process_string_defaults(r#"%convert_case(helloWorld, snake)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_world");

    let result = process_string_defaults(r#"%convert_case(helloWorld, invalid)"#);
    assert!(result.is_err());

    let result = process_string_defaults(r#"%convert_case(text)"#);
    assert!(result.is_err());
}

#[test]
fn test_builtin_to_snake_case() {
    let result = process_string_defaults(r#"%to_snake_case(helloWorld)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_world");

    let result = process_string_defaults(r#"%to_snake_case(HelloWorld)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_world");

    let result = process_string_defaults(r#"%to_snake_case(hello123World)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_123_world");

    let result = process_string_defaults(r#"%to_snake_case(hello_world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_world");
}

#[test]
fn test_builtin_to_camel_case() {
    let result = process_string_defaults(r#"%to_camel_case(hello_world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "helloWorld");

    let result = process_string_defaults(r#"%to_camel_case(HelloWorld)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "helloWorld");

    let result = process_string_defaults(r#"%to_camel_case(hello_123_world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello123World");
}

#[test]
fn test_builtin_to_pascal_case() {
    let result = process_string_defaults(r#"%to_pascal_case(hello_world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "HelloWorld");

    let result = process_string_defaults(r#"%to_pascal_case(helloWorld)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "HelloWorld");

    let result = process_string_defaults(r#"%to_pascal_case(hello_123_world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello123World");
}

#[test]
fn test_builtin_to_screaming_case() {
    let result = process_string_defaults(r#"%to_screaming_case(helloWorld)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "HELLO_WORLD");

    let result = process_string_defaults(r#"%to_screaming_case(hello123World)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "HELLO_123_WORLD");

    let result = process_string_defaults(r#"%to_screaming_case(HELLO_WORLD)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "HELLO_WORLD");
}

#[test]
fn test_case_conversion_edge_cases() {
    let result = process_string_defaults(r#"%to_snake_case()"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "");

    let result = process_string_defaults(r#"%to_pascal_case(a)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "A");

    let result = process_string_defaults(r#"%to_snake_case(hello___world)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "hello_world");

    let result = process_string_defaults(r#"%to_camel_case(XML_HTTP_REQUEST)"#).unwrap();
    assert_eq!(String::from_utf8(result).unwrap().trim(), "xmlHttpRequest");
}
