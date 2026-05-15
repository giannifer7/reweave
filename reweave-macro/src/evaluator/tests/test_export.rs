#[cfg(test)]
mod tests {
    use crate::evaluator::EvalError;
    use crate::macro_api::process_string_defaults;

    #[test]
    fn test_export_plain_copy() {
        // %export now does a plain upward copy — no automatic free-variable freezing.
        // Variables resolved in the macro body (e.g. via %set) are exported as
        // their evaluated string values.  Macros are copied as-is: free variables
        // in the body are not captured, and outer variables do not leak into
        // the later call frame.
        let source = r#"
%def(maker, base, name, %{
    %def(AFILE, param, %(base)/%(name)%(param).txt)
    %export(AFILE)

    %set(my_var, %{from %(base) import %(name)%})
    %export(my_var)
%})
%maker(one, two)
%AFILE(three)
%(my_var)
        "#;
        let err = process_string_defaults(source).unwrap_err();
        assert!(err.to_string().contains("Undefined variable"));
    }

    #[test]
    fn test_export_wrong_number_of_args() {
        let source = "%export(foo, bar)";
        let result = process_string_defaults(source);
        match result {
            Err(EvalError::InvalidUsage(msg)) => {
                assert!(
                    msg.contains("export: exactly 1 arg"),
                    "Unexpected error message: {}",
                    msg
                );
            }
            _ => panic!("Expected an InvalidUsage error when %export is called with two arguments"),
        }
    }
}
