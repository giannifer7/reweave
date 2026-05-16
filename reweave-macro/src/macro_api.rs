use crate::evaluator::output::{EvalOutput, ExpansionLineEntry};
use crate::evaluator::{EvalConfig, EvalError, Evaluator};

pub type TracingResult = (Vec<u8>, Vec<(u32, ExpansionLineEntry)>);
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
fn with_input_context(input_file: &Path, error: EvalError) -> EvalError {
    EvalError::Runtime(format!("{}: {}", input_file.display(), error))
}
pub fn process_string(
    source: &str,
    real_path: Option<&Path>,
    evaluator: &mut Evaluator,
) -> Result<Vec<u8>, EvalError> {
    let path_for_parsing = match real_path {
        Some(rp) => rp.to_path_buf(),
        None => PathBuf::from(format!("<string-{}>", evaluator.num_source_files())),
    };
    let ast = evaluator.parse_string(source, &path_for_parsing)?;
    evaluator.validate_ast_semantics(&ast)?;
    if let Some(rp) = real_path {
        evaluator.set_current_file(rp.to_path_buf());
    }
    let output_string = evaluator.evaluate(&ast)?;
    Ok(output_string.into_bytes())
}
pub fn process_string_tracing(
    source: &str,
    real_path: Option<&Path>,
    evaluator: &mut Evaluator,
) -> Result<TracingResult, EvalError> {
    let path_for_parsing = match real_path {
        Some(rp) => rp.to_path_buf(),
        None => PathBuf::from(format!("<string-{}>", evaluator.num_source_files())),
    };
    let ast = evaluator.parse_string(source, &path_for_parsing)?;
    evaluator.validate_ast_semantics(&ast)?;
    if let Some(rp) = real_path {
        evaluator.set_current_file(rp.to_path_buf());
    }

    let mut out = crate::evaluator::output::TracingOutput::new();
    evaluator.evaluate_to(&ast, &mut out)?;

    let trace_entries = out.into_line_entries(evaluator.sources());
    let output_string = out.finish();

    Ok((output_string.into_bytes(), trace_entries))
}
pub fn process_file_with_writer(
    input_file: &Path,
    writer: &mut dyn Write,
    evaluator: &mut Evaluator,
) -> Result<(), EvalError> {
    let content = fs::read_to_string(input_file)
        .map_err(|e| EvalError::Runtime(format!("Cannot read {input_file:?}: {e}")))?;
    let expanded = process_string(&content, Some(input_file), evaluator)
        .map_err(|e| with_input_context(input_file, e))?;
    writer
        .write_all(&expanded)
        .map_err(|e| EvalError::Runtime(format!("Cannot write to output: {e}")))?;
    Ok(())
}
pub fn process_file(
    input_file: &Path,
    output_file: &Path,
    evaluator: &mut Evaluator,
) -> Result<(), EvalError> {
    ensure_parent_dir(output_file)?;
    let mut file = fs::File::create(output_file)
        .map_err(|e| EvalError::Runtime(format!("Cannot create {output_file:?}: {e}")))?;
    process_file_with_writer(input_file, &mut file, evaluator)
}
pub fn process_files(
    inputs: &[PathBuf],
    output_path: &Path,
    evaluator: &mut Evaluator,
) -> Result<(), EvalError> {
    // Determine the appropriate writer based on output_path
    let mut stdout_handle;
    let mut file_handle;
    let writer: &mut dyn Write = if output_path.to_string_lossy() == "-" {
        stdout_handle = io::stdout();
        &mut stdout_handle
    } else {
        // Create parent directory if needed
        ensure_parent_dir(output_path)?;

        // Open the output file
        file_handle = fs::File::create(output_path)
            .map_err(|e| EvalError::Runtime(format!("Cannot create {output_path:?}: {e}")))?;
        &mut file_handle
    };

    // Process all input files with the selected writer
    for input_path in inputs {
        process_file_with_writer(input_path, writer, evaluator)?;
    }

    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn ensure_parent_dir(output_path: &Path) -> Result<(), EvalError> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| EvalError::Runtime(format!("Cannot create dir {parent:?}: {e}")))?;
    }
    Ok(())
}
pub fn process_files_from_config(
    inputs: &[PathBuf],
    output_dir: &Path,
    config: EvalConfig,
) -> Result<(), EvalError> {
    let mut evaluator = Evaluator::new(config);
    process_files(inputs, output_dir, &mut evaluator)
}
pub fn process_string_defaults(source: &str) -> Result<Vec<u8>, EvalError> {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    process_string(source, None, &mut evaluator)
}
pub fn discover_includes_in_string(
    source: &str,
    real_path: Option<&Path>,
    evaluator: &mut Evaluator,
) -> Result<Vec<PathBuf>, EvalError> {
    evaluator.set_dependency_discovery_active(true);
    let result = process_string(source, real_path, evaluator);
    let includes = evaluator.take_discovered_dependency_paths();
    evaluator.set_dependency_discovery_active(false);
    result.map(|_| includes)
}
pub fn discover_includes_in_file(
    input_file: &Path,
    evaluator: &mut Evaluator,
) -> Result<Vec<PathBuf>, EvalError> {
    let content = fs::read_to_string(input_file)
        .map_err(|e| EvalError::Runtime(format!("Cannot read {input_file:?}: {e}")))?;
    discover_includes_in_string(&content, Some(input_file), evaluator)
        .map_err(|e| with_input_context(input_file, e))
}
/// Evaluate `source` with precise per-byte token attribution.
///
/// Returns the expanded string and a sorted list of `SpanRange` entries —
/// one entry per source-token transition, covering only tracked regions.
/// Use `PreciseTracingOutput::span_at_byte` to query individual positions.
pub fn process_string_precise(
    source: &str,
    real_path: Option<&Path>,
    evaluator: &mut Evaluator,
) -> Result<(String, Vec<crate::evaluator::output::SpanRange>), EvalError> {
    use crate::evaluator::output::PreciseTracingOutput;
    let path_for_parsing = match real_path {
        Some(rp) => rp.to_path_buf(),
        None => PathBuf::from(format!("<string-{}>", evaluator.num_source_files())),
    };
    let ast = evaluator.parse_string(source, &path_for_parsing)?;
    evaluator.validate_ast_semantics(&ast)?;
    if let Some(rp) = real_path {
        evaluator.set_current_file(rp.to_path_buf());
    }
    let mut out = PreciseTracingOutput::new();
    evaluator.evaluate_to(&ast, &mut out)?;
    Ok(out.into_parts())
}
