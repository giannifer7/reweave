#[cfg(test)]
mod tests {
    use crate::macro_api::process_string_defaults;

    #[test]
    fn test_builtin_set() {
        // The %set builtin should set variable "foo" to "bar".
        // Then, the expression "%(foo)" should expand to "bar".
        let source = "%set(foo, bar)%(foo)";
        let result =
            process_string_defaults(source).expect("Failed to process string with %set builtin");
        let output = String::from_utf8(result).expect("Output was not valid UTF-8");
        assert_eq!(output.trim(), "bar");
    }

    #[test]
    fn test_alias_forwards_call() {
        let source = "%def(greet, name, %{Hello %(name)!%})\
                      %alias(say_hi, greet)\
                      %say_hi(world)";
        let result = process_string_defaults(source).unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "Hello world!");
    }

    #[test]
    fn test_alias_independent_after_redef() {
        // Re-defining `greet` after the alias should not affect `say_hi`.
        let source = "%redef(greet, name, %{Hello %(name)!%})\
                      %alias(say_hi, greet)\
                      %redef(greet, name, %{Bye %(name)!%})\
                      %say_hi(world) %greet(world)";
        let result = process_string_defaults(source).unwrap();
        assert_eq!(
            String::from_utf8(result).unwrap(),
            "Hello world! Bye world!"
        );
    }

    #[test]
    fn test_alias_unknown_source_errors() {
        let source = "%alias(foo, no_such_macro)";
        let err = process_string_defaults(source).unwrap_err();
        assert!(err.to_string().contains("not defined"));
    }

    #[test]
    fn test_alias_with_frozen_override() {
        // %(prefix) is a free variable in the body (not a param).
        // The alias pins it to "WARNING" at alias-definition time.
        let source = "%def(render_with_prefix, msg, %{%(prefix): %(msg)%})\
                      %alias(warn, render_with_prefix, prefix = WARNING)\
                      %warn(check this)";
        let result = process_string_defaults(source).unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "WARNING: check this");
    }

    #[test]
    fn test_alias_override_does_not_affect_source() {
        // Pre-binding in the alias must not leak back into the original macro.
        let source = "%def(render_with_prefix, msg, %{%(prefix): %(msg)%})\
                      %alias(warn, render_with_prefix, prefix = WARNING)\
                      %warn(check) %render_with_prefix(check)";
        let err = process_string_defaults(source).unwrap_err();
        assert!(err.to_string().contains("Undefined variable"));
    }

    #[test]
    fn test_alias_override_multiple_vars() {
        let source = "%def(fmt, msg, %{[%(level)] %(tag): %(msg)%})\
                      %alias(err, fmt, level = ERROR, tag = sys)\
                      %err(disk full)";
        let result = process_string_defaults(source).unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "[ERROR] sys: disk full");
    }

    #[test]
    fn test_alias_override_args_must_be_named() {
        let source = "%def(foo, x, %{%(x)%})\
                      %alias(bar, foo, not_named)";
        let err = process_string_defaults(source).unwrap_err();
        assert!(err.to_string().contains("named"));
    }

    #[test]
    fn test_set_export_alias_and_eval_reject_invalid_shapes() {
        for (source, expected) in [
            ("%set(foo)", "exactly 2 args"),
            ("%export()", "exactly 1 arg"),
            ("%alias(one)", "at least 2 args"),
            ("%def(foo, x, %(x))%alias(new = nope, foo)", "positional"),
            ("%def(foo, x, %(x))%alias(def, foo)", "built-in"),
            ("%eval()", "requires macroName"),
        ] {
            let err = process_string_defaults(source).unwrap_err();
            assert!(
                err.to_string().contains(expected),
                "expected {expected:?} in {err}"
            );
        }
    }

    #[test]
    fn test_eval_with_empty_computed_name_is_noop() {
        let result = process_string_defaults("%eval(%if(, never))after").unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "after");
    }
}
