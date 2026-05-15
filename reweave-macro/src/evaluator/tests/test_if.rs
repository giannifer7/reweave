use crate::macro_api::process_string_defaults;

#[test]
fn test_if_condition_true() {
    let result = process_string_defaults(
        r#"
        %if(true, %{
            This should be printed.
        %})
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "This should be printed."
    );
}

#[test]
fn test_if_condition_false() {
    let result = process_string_defaults(
        r#"
        %if(  , %{
            This should NOT be printed.
        %})
        "#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "");
}

#[test]
fn test_if_else_condition_true() {
    let result = process_string_defaults(
        r#"
        %if(true, %{
            This should be printed.
        %}, %{
            This should NOT be printed.
        %})
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "This should be printed."
    );
}

#[test]
fn test_if_else_condition_false() {
    let result = process_string_defaults(
        r#"
        %if(, %{
            This should NOT be printed.
        %}, %{
            This should be printed.
        %})
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "This should be printed."
    );
}

#[test]
fn test_nested_if_conditions() {
    let result = process_string_defaults(
        r#"
        %if(true, %{
            %if(true, %{
                Nested condition is true.
            %})
        %})
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "Nested condition is true."
    );
}

#[test]
fn test_if_with_macro_condition() {
    let result = process_string_defaults(
        r#"
        %def(empty, %{%})
        %if(%empty(), %{condition is true.%}, %{condition is false.%})
        "#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "condition is false."
    );
}

#[test]
fn test_match_selects_first_regex_branch() {
    let result = process_string_defaults(
        r#"%match(error-404, fallback,
                  ^warn-\d+$, warn,
                  ^error-\d+$, error,
                  .*, other)"#,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "error");
}

#[test]
fn test_match_uses_default_when_no_regex_matches() {
    let result = process_string_defaults("%match(info, default, ^error$, error)").unwrap();
    assert_eq!(result, b"default");
}

#[test]
fn test_match_default_is_lazy_when_branch_matches() {
    let result =
        process_string_defaults("%match(warn, %undefined_default(), ^warn$, selected)").unwrap();

    assert_eq!(result, b"selected");
}

#[test]
fn test_match_is_lazy_in_value_branches() {
    let result = process_string_defaults(
        "%match(error, default, ^error$, selected, ^warn$, %undefined_macro())",
    )
    .unwrap();

    assert_eq!(result, b"selected");
}

#[test]
fn test_match_regexes_after_match_are_not_evaluated() {
    let result = process_string_defaults(
        "%match(error, default, ^error$, selected, %undefined_pattern(), unreachable)",
    )
    .unwrap();

    assert_eq!(result, b"selected");
}

#[test]
fn test_match_invalid_regex_after_match_is_not_evaluated() {
    let result = process_string_defaults(
        "%match(error, default, ^error$, selected, %(bad_regex), unreachable)",
    )
    .unwrap();

    assert_eq!(result, b"selected");
}

#[test]
fn test_match_pattern_can_be_macro_generated() {
    let result = process_string_defaults(
        "%def(kind_pattern, kind, %{^%(kind)-\\d+$%})%match(warn-12, no, %kind_pattern(warn), yes)",
    )
    .unwrap();

    assert_eq!(result, b"yes");
}

#[test]
fn test_match_exposes_numbered_and_named_captures() {
    let result = process_string_defaults(
        r#"%match(issue-42, no,
                  %[^(?P<prefix>[a-z]+)-(\d+)$%],
                  %{%(prefix):%(match_1):%(match_2):%(match_0)%})"#,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(result).unwrap().trim(),
        "issue:issue:42:issue-42"
    );
}

#[test]
fn test_match_rejects_duplicate_named_capture_regex() {
    let result = process_string_defaults(
        r#"%match(abc-123, no,
                  %[^(?P<value>[a-z]+)-(?P<value>\d+)$%],
                  %{%(value)%})"#,
    );

    assert!(
        result.is_err(),
        "duplicate regex capture names should be rejected by the regex parser"
    );
}

#[test]
fn test_match_capture_variables_do_not_leak() {
    let result = process_string_defaults(
        "%set(match_1, outer)%match(a, no, %[(a)%], %{%(match_1)%})/%(match_1)",
    )
    .unwrap();

    assert_eq!(result, b"a/outer");
}

#[test]
fn test_match_named_capture_restores_existing_variable() {
    let result = process_string_defaults(
        "%set(kind, outer)%match(error-500, no, %[^(?P<kind>[a-z]+)-\\d+$%], %{%(kind)%})/%(kind)",
    )
    .unwrap();

    assert_eq!(result, b"error/outer");
}

#[test]
fn test_match_unmatched_optional_capture_is_absent() {
    let err =
        process_string_defaults("%match(error, no, %[^(error)(?:-(\\d+))?$%], %{%(match_2)%})")
            .unwrap_err();

    assert!(
        err.to_string().contains("Undefined variable"),
        "unmatched optional capture should not bind match_2: {err}"
    );
}

#[test]
fn test_match_invalid_regex_errors_when_reached() {
    let err = process_string_defaults("%match(value, default, (, broken)").unwrap_err();
    assert!(err.to_string().contains("invalid regex"), "{err}");
}

#[test]
fn test_match_requires_regex_value_pairs() {
    let err = process_string_defaults("%match(value, default, ^value$)").unwrap_err();
    assert!(err.to_string().contains("regex/value"), "{err}");
}

#[test]
fn test_match_requires_at_least_value_and_default() {
    let err = process_string_defaults("%match(value)").unwrap_err();
    assert!(err.to_string().contains("at least"), "{err}");
}
