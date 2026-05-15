use crate::macro_api::process_string_defaults;

#[test]
fn test_simple_variable_substitution() {
    let result = process_string_defaults(
        r#"
        %def(greet, name, %{
            Hello, %(name)!
        %})
        %greet(World)
        "#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello, World!");
}

#[test]
fn test_nested_variable_substitution() {
    let result = process_string_defaults(
        r#"
        %def(greet, name, %{
            Hello, %(name)!
        %})
        %def(greet_twice, name, %{
            %greet(%(name))
            %greet(%(name))
        %})
        %greet_twice(World)
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "Hello, World!\n        \n            \n            Hello, World!"
    );
}

#[test]
fn test_variable_substitution_with_whitespace() {
    let result = process_string_defaults(
        r#"
        %def(greet, name, %{
            Hello, %(name)!
        %})
        %greet(  World  )
        "#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello, World  !");
}

#[test]
fn test_variable_substitution_with_missing_param_is_error() {
    let err = process_string_defaults(
        r#"
        %def(greet, name, %{
            Hello, %(name)!
        %})
        %greet()
        "#,
    )
    .unwrap_err();

    assert!(err.to_string().contains("Unbound parameter"));
}

#[test]
fn test_variable_substitution_with_multiple_arguments() {
    let result = process_string_defaults(
        r#"
        %def(greet, name, greeting, %{
            %(greeting), %(name)!
        %})
        %greet(World, Hello)
        "#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello, World!");
}

#[test]
fn test_variable_substitution_with_macro_in_argument() {
    let result = process_string_defaults(
        r#"
        %def(get_name, %{
            World
        %})
        %def(greet, name, %{
            Hello, %(name)!
        %})
        %greet(%get_name())
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "Hello, \n            World\n        !"
    );
}

#[test]
fn test_variable_substitution_with_conditional_logic() {
    let result = process_string_defaults(
        r#"
        %def(greet, name, %{
            %if(%(name), %{
                Hello, %(name)!
            %}, %{
                Hello, stranger!
            %})
        %})
        %greet(World)
        "#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Hello, World!");
}

#[test]
fn test_variable_substitution_with_conditional_logic_missing_param_is_error() {
    let err = process_string_defaults(
        r#"
        %def(greet, name, %{
            %if(%(name), %{
                Hello, %(name)!
            %}, %{
                Hello, stranger!
            %})
        %})
        %greet()
        "#,
    )
    .unwrap_err();

    assert!(err.to_string().contains("Unbound parameter"));
}
