use crate::evaluator::output::{EvalOutput, PlainOutput, PreciseTracingOutput, SourceSpan};
use crate::evaluator::{EvalConfig, Evaluator};
use crate::macro_api::process_string_defaults;
use crate::types::{ASTNode, NodeKind, Token, TokenKind};
use std::path::PathBuf;

/// Helper: evaluate `source` through evaluate_to(PlainOutput) and return the
/// resulting String.
fn eval_to_plain(source: &str) -> String {
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("<test>");
    let ast = eval.parse_string(source, &path).unwrap();
    let mut out = PlainOutput::new();
    eval.evaluate_to(&ast, &mut out).unwrap();
    out.finish()
}

// ---------- Parity tests: evaluate_to(PlainOutput) == evaluate() ----------

#[test]
fn plain_output_parity_literal_text() {
    let src = "hello world";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_variable() {
    let src = "%set(x, 42)%(x) is the answer";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_def_and_call() {
    let src = "%def(greet, name, %{Hello, %(name)!%})%greet(World)";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_nested_macros() {
    let src = r#"%def(inner, %{X%})%def(outer, %{[%inner()]%})%outer()"#;
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_if() {
    let src = "%set(flag, yes)%if(%(flag), true, false)";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_multiline() {
    let src = "%def(tag, name, value, %{<%(name)>%(value)</%(name)>%})\n%tag(div, hello)\n";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_parity_named_args() {
    let src = "%def(tag, name, value, %{<%(name)>%(value)</%(name)>%})%tag(name=span, value=hi)";
    let via_evaluate = String::from_utf8(process_string_defaults(src).unwrap()).unwrap();
    let via_output = eval_to_plain(src);
    assert_eq!(via_output, via_evaluate);
}

#[test]
fn plain_output_default_collects_tracked_and_untracked_text() {
    let mut out = PlainOutput::default();
    let span = SourceSpan {
        src: 0,
        pos: 0,
        length: 0,
        kind: SpanKind::Literal,
    };

    out.push_str("tracked", span);
    out.push_untracked("-untracked");

    assert_eq!(out.finish(), "tracked-untracked");
}

#[test]
fn evaluate_comment_nodes_are_noops_for_plain_and_output_paths() {
    let comment = ASTNode {
        kind: NodeKind::LineComment,
        src: 0,
        token: Token {
            kind: TokenKind::LineComment,
            src: 0,
            pos: 0,
            length: 9,
        },
        end_pos: 9,
        parts: Vec::new(),
        name: None,
    };
    let mut eval = Evaluator::new(EvalConfig::default());

    assert_eq!(eval.evaluate(&comment).unwrap(), "");

    let mut out = PlainOutput::new();
    eval.evaluate_to(&comment, &mut out).unwrap();
    assert_eq!(out.finish(), "");
}

// ---------- Span correctness: SpyOutput to verify spans ----------

/// A test-only EvalOutput that records spans.
struct SpyOutput {
    buf: String,
    spans: Vec<(String, SourceSpan)>,
    untracked: Vec<String>,
}

impl SpyOutput {
    fn new() -> Self {
        Self {
            buf: String::new(),
            spans: Vec::new(),
            untracked: Vec::new(),
        }
    }
}

impl EvalOutput for SpyOutput {
    fn push_str(&mut self, text: &str, span: SourceSpan) {
        self.buf.push_str(text);
        self.spans.push((text.to_string(), span));
    }

    fn push_untracked(&mut self, text: &str) {
        self.buf.push_str(text);
        self.untracked.push(text.to_string());
    }

    fn finish(self) -> String {
        self.buf
    }
}

#[test]
fn spy_output_captures_literal_spans() {
    let src = "abc";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("<test>");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut spy = SpyOutput::new();
    eval.evaluate_to(&ast, &mut spy).unwrap();

    assert_eq!(spy.buf, "abc");
    assert!(!spy.spans.is_empty(), "no spans were recorded");
    let (text, span) = &spy.spans[0];
    assert_eq!(text, "abc");
    assert_eq!(span.pos, 0);
    assert_eq!(span.length, 3);
}

#[test]
fn spy_output_set_variable_lookup_is_untracked() {
    // %set stores a variable with no span; %(x) therefore goes through
    // push_untracked (no origin token to attribute).  Note: %set itself
    // returns "" so it produces no output at all.
    let src = "%set(x, val)%(x)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("<test>");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut spy = SpyOutput::new();
    eval.evaluate_to(&ast, &mut spy).unwrap();

    assert_eq!(spy.buf, "val");
    let untracked_texts: Vec<&str> = spy.untracked.iter().map(|t| t.as_str()).collect();
    assert!(
        untracked_texts.contains(&"val"),
        "variable expansion without origin span should be untracked, got: {:?}",
        untracked_texts
    );
}

#[test]
fn spy_output_builtin_result_is_tracked_as_computed() {
    // Builtins that return non-empty strings (e.g. %capitalize) must emit
    // push_str with SpanKind::Computed, not push_untracked.
    // This ensures the tracer can attribute the output line to the call site.
    let src = "%capitalize(hello)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("<test>");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut spy = SpyOutput::new();
    eval.evaluate_to(&ast, &mut spy).unwrap();

    assert_eq!(spy.buf, "Hello");
    assert!(
        spy.untracked.is_empty(),
        "builtin result should not be untracked"
    );
    let (text, span) = spy
        .spans
        .iter()
        .find(|(t, _)| t == "Hello")
        .expect("tracked span for 'Hello' not found");
    assert_eq!(text, "Hello");
    assert_eq!(
        span.kind,
        SpanKind::Computed,
        "builtin result should carry SpanKind::Computed"
    );
}

#[test]
fn spy_output_user_macro_is_tracked() {
    let src = "%def(wrap, x, %{[%(x)]%})%wrap(hi)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("<test>");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut spy = SpyOutput::new();
    eval.evaluate_to(&ast, &mut spy).unwrap();

    assert_eq!(spy.buf, "[hi]");
    let tracked_texts: Vec<&str> = spy.spans.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        tracked_texts.contains(&"["),
        "literal '[' in macro body should be tracked"
    );
    assert!(
        tracked_texts.contains(&"]"),
        "literal ']' in macro body should be tracked"
    );
    assert!(
        tracked_texts.contains(&"hi"),
        "argument substitution should be tracked"
    );
}

// ---------- TracingOutput tests ----------

use crate::evaluator::output::SpanKind;
use crate::evaluator::output::TracingOutput;

#[test]
fn tracing_output_single_line_gets_an_entry() {
    let src = "%def(wrap, x, %{[%(x)]%})%wrap(hi)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("test.md");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut out = TracingOutput::new();
    eval.evaluate_to(&ast, &mut out).unwrap();

    let entries = out.into_line_entries(eval.sources());
    assert_eq!(out.finish(), "[hi]");
    assert_eq!(
        entries.len(),
        1,
        "expected one line entry, got: {entries:?}"
    );
    let (out_line, entry) = &entries[0];
    assert_eq!(*out_line, 0);
    assert!(
        matches!(&entry.kind, SpanKind::MacroBody { macro_name } if macro_name == "wrap"),
        "expected MacroBody(wrap), got: {:?}",
        entry.kind
    );
}

#[test]
fn test_line_entries() {
    let src = "line 1\nline 2 with %set(x, val)%(x)\nline 3";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("test.md");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut out = TracingOutput::new();
    eval.evaluate_to(&ast, &mut out).unwrap();

    let entries = out.into_line_entries(eval.sources());

    assert_eq!(entries.len(), 3);

    let (out_line_0, entry_0) = &entries[0];
    assert_eq!(*out_line_0, 0);
    assert!(entry_0.src_file.ends_with("test.md"));
    assert_eq!(entry_0.src_line, 0);
    assert_eq!(entry_0.src_col, 0);
    assert!(matches!(entry_0.kind, SpanKind::Literal));

    let (out_line_1, entry_1) = &entries[1];
    assert_eq!(*out_line_1, 1);
    assert_eq!(entry_1.src_line, 1);

    let (out_line_2, entry_2) = &entries[2];
    assert_eq!(*out_line_2, 2);
    assert_eq!(entry_2.src_line, 2);
}

#[test]
fn tracing_output_tracks_multiline_variable_without_origin_spans() {
    let src = "%set(x, a\nb)%(x)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("vars.md");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut out = PreciseTracingOutput::new();

    eval.evaluate_to(&ast, &mut out).unwrap();

    let (output, ranges) = out.into_parts();
    assert_eq!(output, "a\nb");
    assert!(ranges.iter().any(
        |range| matches!(range.span.kind, SpanKind::VarBinding { ref var_name } if var_name == "x")
    ));
}

#[test]
fn evaluate_to_reports_undefined_variables() {
    let src = "%(missing)";
    let mut eval = Evaluator::new(EvalConfig::default());
    let path = PathBuf::from("missing.md");
    let ast = eval.parse_string(src, &path).unwrap();
    let mut out = PlainOutput::new();

    let err = eval.evaluate_to(&ast, &mut out).unwrap_err();

    assert!(err.to_string().contains("missing"));
}
