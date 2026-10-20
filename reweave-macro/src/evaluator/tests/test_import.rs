#[cfg(test)]
mod tests {
    use crate::evaluator::tests::test_utils::evaluator_in_temp_dir;
    use crate::macro_api::{process_string, process_string_defaults};
    use std::fs;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

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

    #[test]
    fn test_repeat_import_of_same_path_is_a_no_op() {
        // Two separate top-level documents, processed against the same evaluator (as reweave-cli
        // does for every input in one --dir batch), both %import the same shared fragment. The
        // second import must not re-run %def and fail as a duplicate binding.
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        fs::write(temp_dir.path().join("shared.md"), "%def(shout, x, %(x)!)\n")
            .expect("Failed to write shared.md");
        let mut evaluator = evaluator_in_temp_dir(temp_dir.path());

        let doc_a = "%import(shared.md)\n%shout(a)";
        let doc_b = "%import(shared.md)\n%shout(b)";

        let out_a = process_string(doc_a, None, &mut evaluator).expect("doc_a failed");
        let out_b = process_string(doc_b, None, &mut evaluator).expect("doc_b failed");

        assert_eq!(String::from_utf8(out_a).unwrap().trim(), "a!");
        assert_eq!(String::from_utf8(out_b).unwrap().trim(), "b!");
    }
}
