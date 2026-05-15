#[cfg(test)]
mod tests {
    use crate::macro_api::process_string_defaults;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_import_includes_definitions_only() {
        let mut tmp = NamedTempFile::new().expect("Failed to create temporary file");
        writeln!(tmp, "This text should be discarded.").expect("Failed to write to temp file");
        writeln!(tmp, "%def(included_macro, x, included: %(x)!)")
            .expect("Failed to write macro definition to temp file");
        writeln!(tmp, "More text that should be discarded.").expect("Failed to write to temp file");
        let tmp_path = tmp
            .path()
            .to_str()
            .expect("Temporary file path is not valid UTF-8");

        let source = format!(
            r#"
%def(macro_using_includes, param, %{{
    %import({tmp_path})
    %included_macro(%(param))
%}})
%macro_using_includes(test)
"#
        );

        let result = process_string_defaults(&source);
        match result {
            Ok(output) => {
                let output_str = String::from_utf8(output).expect("Output was not valid UTF-8");
                let expected = "included: test!";
                assert_eq!(
                    output_str.trim(),
                    expected,
                    "Output did not match expected result. Got: {:?}",
                    output_str
                );
            }
            Err(e) => {
                panic!("Processing failed with error: {:?}", e);
            }
        }
    }
}
