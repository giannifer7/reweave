use crate::evaluator::{EvalConfig, Evaluator};
use crate::evaluator::output::{PreciseTracingOutput, SpanKind};
use crate::macro_api::{
    discover_includes_in_string, process_file, process_files, process_files_from_config,
    process_string, process_string_defaults, process_string_precise, process_string_tracing,
};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = fs::File::create(&path).unwrap();
    write!(file, "{}", content).unwrap();
    path
}

#[test]
fn test_process_string_basic() {
    let result = process_string_defaults("Hello %def(test, World) %test()").unwrap();
    assert_eq!(String::from_utf8(result).unwrap(), "Hello  World");
}

#[test]
fn test_include_basic() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let _include_file = create_temp_file(&temp_dir, "include.txt", "test");

    let main_file = create_temp_file(&temp_dir, "main.txt", "%include(include.txt)");

    let config = EvalConfig {
        include_paths: vec![temp_dir.path().to_path_buf()],
        ..EvalConfig::default()
    };
    let mut evaluator = Evaluator::new(config);

    let output_file = temp_dir.path().join("output.txt");

    process_file(&main_file, &output_file, &mut evaluator)?;

    let result = fs::read_to_string(output_file)?;
    assert_eq!(result.trim(), "test");

    Ok(())
}

#[test]
fn test_process_string_with_error() {
    let result = process_string_defaults("%undefined_macro()");
    assert!(result.is_err());
}

#[test]
fn test_process_string_with_nested_macros() {
    let source = r#"
        %def(inner, value, Inside: %(value))
        %def(outer, arg, Outside: %inner(%(arg)))
        %outer(test)
    "#;

    let result = process_string_defaults(source).unwrap();
    let output = String::from_utf8(result).unwrap();
    assert!(output.contains("Outside: Inside: test"));
}

#[test]
fn test_process_string_with_sigil_chars() {
    let config = EvalConfig {
        sigil: '@',
        ..EvalConfig::default()
    };
    let mut evaluator = Evaluator::new(config);

    let result = process_string(
        "@def(test, value, Result: @(value))@test(works)",
        None,
        &mut evaluator,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Result: works");
}

#[test]
fn test_process_string_with_unicode_sigil() {
    let config = EvalConfig {
        sigil: '§',
        ..EvalConfig::default()
    };
    let mut evaluator = Evaluator::new(config);

    let result = process_string(
        "§def(test, value, Result: §(value))§test(works)",
        None,
        &mut evaluator,
    )
    .unwrap();

    assert_eq!(String::from_utf8(result).unwrap().trim(), "Result: works");
}

#[test]
fn test_process_files_with_shared_macros() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", "%def(shared, Shared content)");
    let file2 = create_temp_file(&temp_dir, "file2.txt", "%shared()");

    let output_file = temp_dir.path().join("output.txt");

    let config = EvalConfig::default();
    process_files_from_config(&[file1, file2], &output_file, config).unwrap();

    let output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(output.trim(), "Shared content");
}

#[test]
fn test_process_files_can_write_to_existing_parent() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_temp_file(&temp_dir, "input.txt", "plain text");
    let output_file = temp_dir.path().join("nested").join("output.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    process_files(&[input], &output_file, &mut evaluator).unwrap();

    assert_eq!(fs::read_to_string(output_file).unwrap(), "plain text");
}

#[test]
fn test_process_file_wraps_input_context_on_eval_error() {
    let temp_dir = TempDir::new().unwrap();
    let input = create_temp_file(&temp_dir, "bad.txt", "%undefined()");
    let output = temp_dir.path().join("out.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let err = process_file(&input, &output, &mut evaluator).unwrap_err();
    assert!(err.to_string().contains("bad.txt"));
}

#[test]
fn test_process_string_tracing_records_literal_lines() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let (bytes, entries) =
        process_string_tracing("alpha\nbeta\n", None, &mut evaluator).unwrap();

    assert_eq!(String::from_utf8(bytes).unwrap(), "alpha\nbeta\n");
    assert_eq!(entries.len(), 2);
    assert!(matches!(entries[0].1.kind, SpanKind::Literal));
    assert_eq!(entries[0].1.src_line, 0);
    assert_eq!(entries[1].1.src_line, 1);
}

#[test]
fn test_process_string_tracing_uses_real_path_when_provided() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("source.rew");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let (bytes, entries) = process_string_tracing("alpha", Some(&path), &mut evaluator).unwrap();

    assert_eq!(String::from_utf8(bytes).unwrap(), "alpha");
    assert_eq!(evaluator.get_current_file_path(), path);
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_process_string_precise_tracks_literal_and_macro_argument_spans() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let (output, ranges) =
        process_string_precise("%def(wrap, x, <%(x)>)%wrap(value)", None, &mut evaluator).unwrap();

    assert_eq!(output, "<value>");
    assert!(
        ranges
            .iter()
            .any(|range| matches!(range.span.kind, SpanKind::MacroBody { .. }))
    );
    assert!(
        ranges
            .iter()
            .any(|range| matches!(range.span.kind, SpanKind::MacroArg { .. }))
    );
    assert!(PreciseTracingOutput::span_at_byte(&ranges, 1).is_some());
    assert!(PreciseTracingOutput::span_at_byte(&ranges, output.len()).is_none());
}

#[test]
fn test_process_string_precise_uses_real_path_when_provided() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("precise.rew");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let (output, ranges) = process_string_precise("alpha", Some(&path), &mut evaluator).unwrap();

    assert_eq!(output, "alpha");
    assert_eq!(evaluator.get_current_file_path(), path);
    assert!(!ranges.is_empty());
}

#[test]
fn test_process_string_precise_tracks_named_arguments_and_alias_overrides() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let (output, ranges) = process_string_precise(
        "%def(pair, left, %(left)-%(right))%alias(pair_right, pair, right=frozen)%pair_right(left=value)",
        None,
        &mut evaluator,
    )
    .unwrap();

    assert_eq!(output, "value-frozen");
    assert!(
        ranges
            .iter()
            .any(|range| matches!(range.span.kind, SpanKind::MacroArg { ref param_name, .. } if param_name == "left"))
    );
    assert!(
        ranges
            .iter()
            .any(|range| matches!(range.span.kind, SpanKind::VarBinding { ref var_name } if var_name == "right"))
    );
}

#[test]
fn test_process_string_tracing_records_computed_builtin_output() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let (bytes, entries) = process_string_tracing("%capitalize(word)", None, &mut evaluator).unwrap();

    assert_eq!(String::from_utf8(bytes).unwrap(), "Word");
    assert!(
        entries
            .iter()
            .any(|(_, entry)| matches!(entry.kind, SpanKind::Computed))
    );
}

#[test]
fn test_process_string_tracing_propagates_macro_call_errors() {
    let mut evaluator = Evaluator::new(EvalConfig::default());

    assert!(process_string_tracing("%missing()", None, &mut evaluator).is_err());

    let mut evaluator = Evaluator::new(EvalConfig::default());
    assert!(
        process_string_tracing("%def(one, x, %(x))%one()", None, &mut evaluator).is_err()
    );
}

#[test]
fn test_discover_includes_in_string_evaluates_conditionals() {
    let temp_dir = TempDir::new().unwrap();
    let included = create_temp_file(&temp_dir, "fragment.txt", "fragment");
    let config = EvalConfig {
        include_paths: vec![temp_dir.path().to_path_buf()],
        ..EvalConfig::default()
    };
    let mut evaluator = Evaluator::new(config);

    let paths = discover_includes_in_string(
        "%def(path, fragment.txt)%if(1, %include(%path()),)",
        None,
        &mut evaluator,
    )
    .unwrap();

    assert_eq!(paths, vec![included]);
}
