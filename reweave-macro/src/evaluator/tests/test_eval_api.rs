use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};

use crate::evaluator::{
    core::Evaluator,
    errors::EvalError,
    eval_api::{
        eval_file, eval_file_with_config, eval_files, eval_files_with_config, eval_string,
        eval_string_with_defaults,
    },
    state::EvalConfig,
};

fn create_temp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file
}

#[test]
fn test_eval_string_basic() {
    let result = eval_string_with_defaults("%def(hello, World)\nHello %hello()!").unwrap();
    assert_eq!(result, "\nHello World!");
}

#[test]
fn test_eval_file() {
    let temp_dir = TempDir::new().unwrap();

    let input_content = "%def(greeting, Hello)\n%greeting(), World!";
    let input_file = create_temp_file(input_content);

    let output_file = temp_dir.path().join("output.txt");

    eval_file_with_config(input_file.path(), &output_file, EvalConfig::default()).unwrap();

    let result = std::fs::read_to_string(output_file).unwrap();
    assert_eq!(result, "\nHello, World!");
}

#[test]
fn test_eval_file_overwrite_protection() {
    let input_file = create_temp_file("content");
    let result = eval_file_with_config(input_file.path(), input_file.path(), EvalConfig::default());
    assert!(matches!(result, Err(EvalError::Runtime(_))));
}

#[test]
fn test_eval_files() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("output");

    let file1 = create_temp_file("%def(shared, Shared content)");
    let file2 = create_temp_file("%shared()");

    eval_files_with_config(
        &[file1.path().to_path_buf(), file2.path().to_path_buf()],
        &output_dir,
        EvalConfig::default(),
    )
    .unwrap();

    let files: Vec<_> = std::fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 2);
}

#[test]
fn eval_string_with_real_path_sets_current_file() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let fake_path = Path::new("/tmp/fake_source.adoc");
    let result = eval_string("%def(x, hi)\n%x()", Some(fake_path), &mut evaluator).unwrap();
    assert_eq!(result, "\nhi");
}

#[test]
fn eval_string_without_path_uses_placeholder() {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let result = eval_string("hello world", None, &mut evaluator).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn eval_file_errors_on_missing_input() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nonexistent.adoc");
    let out = tmp.path().join("out.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());
    let err = eval_file(&missing, &out, &mut evaluator).unwrap_err();
    assert!(err.to_string().contains("Cannot resolve input path"));
}

#[test]
fn eval_file_errors_when_output_parent_cannot_be_resolved() {
    let tmp = TempDir::new().unwrap();
    let input = create_temp_file("content");
    let out = tmp.path().join("missing-parent").join("out.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let err = eval_file(input.path(), &out, &mut evaluator).unwrap_err();

    assert!(err.to_string().contains("Cannot resolve output path"));
}

#[test]
fn eval_file_reports_create_dir_failure_after_canonical_checks() {
    let tmp = TempDir::new().unwrap();
    let input = create_temp_file("content");
    let blocked = tmp.path().join("blocked");
    std::fs::write(&blocked, "not a directory").unwrap();
    let out = blocked.join("out.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let err = eval_file(input.path(), &out, &mut evaluator).unwrap_err();

    assert!(err.to_string().contains("Cannot create dir"));
}

#[test]
fn eval_files_shared_evaluator_sees_prior_defs() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");

    let mut f1 = NamedTempFile::new().unwrap();
    write!(f1, "%def(greeting, Hi)").unwrap();
    let mut f2 = NamedTempFile::new().unwrap();
    write!(f2, "%greeting()").unwrap();

    let mut evaluator = Evaluator::new(EvalConfig::default());
    eval_files(
        &[f1.path().to_path_buf(), f2.path().to_path_buf()],
        &out_dir,
        &mut evaluator,
    )
    .unwrap();

    let files: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 2);
}

#[test]
fn eval_files_uses_fallback_name_for_paths_without_file_name() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let err = eval_files(&[PathBuf::from("/")], &out_dir, &mut evaluator).unwrap_err();

    assert!(err.to_string().contains("Cannot read"));
}

#[test]
fn eval_file_writes_output_to_existing_dir() {
    let tmp = TempDir::new().unwrap();
    let input = create_temp_file("plain text");
    let out = tmp.path().join("output.txt");
    let mut evaluator = Evaluator::new(EvalConfig::default());
    eval_file(input.path(), &out, &mut evaluator).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "plain text");
}

#[test]
fn eval_file_reports_write_errors() {
    let tmp = TempDir::new().unwrap();
    let input = create_temp_file("plain text");
    let output_dir = tmp.path().join("output-dir");
    std::fs::create_dir(&output_dir).unwrap();
    let mut evaluator = Evaluator::new(EvalConfig::default());

    let err = eval_file(input.path(), &output_dir, &mut evaluator).unwrap_err();

    assert!(err.to_string().contains("Cannot write"));
}

#[test]
fn eval_files_with_config_creates_output_dir() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("new_output_dir");
    let input = create_temp_file("hello");
    eval_files_with_config(
        &[input.path().to_path_buf()],
        &out_dir,
        EvalConfig::default(),
    )
    .unwrap();
    assert!(out_dir.exists());
}
