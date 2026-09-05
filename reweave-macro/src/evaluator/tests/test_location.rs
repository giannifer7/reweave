//! Tests for source locations attached to evaluation errors.

use crate::evaluator::{EvalError, SourceLocation};
use crate::macro_api::process_string;
use crate::evaluator::{EvalConfig, Evaluator};

use std::path::Path;

fn eval_file(path: &Path, source: &str) -> Result<Vec<u8>, EvalError> {
    let mut evaluator = Evaluator::new(EvalConfig::default());
    process_string(source, Some(path), &mut evaluator)
}

#[test]
fn undefined_variable_reports_file_and_line() {
    let path = Path::new("example.md");
    let err = eval_file(path, "first line\nsecond %(missing) line\n").unwrap_err();

    match err {
        EvalError::UndefinedVariable { name, location } => {
            assert_eq!(name, "missing");
            let loc = location.expect("location should be attached");
            assert_eq!(loc.file, "example.md");
            assert_eq!(loc.line, 2);
            assert!(loc.col > 0);
        }
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

#[test]
fn undefined_macro_reports_file_and_line() {
    let path = Path::new("example.md");
    let err = eval_file(path, "intro\n%nope()\n").unwrap_err();

    match err {
        EvalError::UndefinedMacro { name, location } => {
            assert_eq!(name, "nope");
            let loc = location.expect("location should be attached");
            assert_eq!(loc.file, "example.md");
            assert_eq!(loc.line, 2);
        }
        other => panic!("expected UndefinedMacro, got {other:?}"),
    }
}

#[test]
fn error_display_includes_location_prefix() {
    let path = Path::new("example.md");
    let err = eval_file(path, "%(missing)\n").unwrap_err();
    let rendered = err.to_string();

    assert!(
        rendered.starts_with("example.md:1:"),
        "expected 'example.md:1:…' prefix, got: {rendered}"
    );
    assert!(rendered.contains("Undefined variable: missing"));
}

#[test]
fn error_display_without_location_is_unchanged() {
    let err = EvalError::UndefinedVariable {
        name: "x".to_string(),
        location: None,
    };
    assert_eq!(err.to_string(), "Undefined variable: x");
}

#[test]
fn source_location_displays_as_file_line_col() {
    let loc = SourceLocation {
        file: "a.md".to_string(),
        line: 3,
        col: 7,
    };
    assert_eq!(loc.to_string(), "a.md:3:7");
}

#[test]
fn builtin_errors_get_call_site_location_but_keep_variant() {
    let path = Path::new("example.md");
    let err = eval_file(path, "intro\n%convert_case(one)\n").unwrap_err();

    assert!(matches!(err, EvalError::InvalidUsage(..)));
    let rendered = err.to_string();
    assert!(
        rendered.starts_with("example.md:2:"),
        "expected location-first rendering, got: {rendered}"
    );
    assert!(rendered.contains("Invalid usage: convert_case: exactly 2 args"));
}

#[test]
fn unbound_parameter_gets_call_site_location() {
    let path = Path::new("example.md");
    let err = eval_file(path, "%def(greet, name, %{hi%})\n%greet()\n").unwrap_err();

    match err {
        EvalError::UnboundParameter { location, .. } => {
            let loc = location.expect("location should be attached");
            assert_eq!(loc.file, "example.md");
            assert_eq!(loc.line, 2);
        }
        other => panic!("expected UnboundParameter, got {other:?}"),
    }
}

#[test]
fn inner_location_wins_over_outer_call_site() {
    // %(missing) inside the %outer body is resolved at its own position,
    // not at the %outer() call site.
    let path = Path::new("example.md");
    let source = "%def(outer, %{%(missing)%})\n%outer()\n";
    let err = eval_file(path, source).unwrap_err();

    match err {
        EvalError::UndefinedVariable { location, .. } => {
            let loc = location.expect("location should be attached");
            assert_eq!(loc.file, "example.md");
            assert_eq!(loc.line, 1);
        }
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

fn test_loc() -> SourceLocation {
    SourceLocation {
        file: "f.md".to_string(),
        line: 1,
        col: 1,
    }
}

#[test]
fn ensure_location_prefixes_string_variants() {
    for err in [
        EvalError::BuiltinError(None, "b".to_string()),
        EvalError::InvalidUsage(None, "u".to_string()),
        EvalError::Runtime(None, "r".to_string()),
        EvalError::IncludeNotFound(None, "i".to_string()),
        EvalError::CircularInclude(None, "c".to_string()),
        EvalError::ParseError(None, "p".to_string()),
    ] {
        let located = err.ensure_location(Some(test_loc()));
        assert!(
            located.to_string().contains("f.md:1:1"),
            "missing location in: {located}"
        );
    }
}

#[test]
fn ensure_location_fills_empty_slots_and_keeps_existing() {
    let filled = EvalError::UndefinedVariable {
        name: "x".to_string(),
        location: None,
    }
    .ensure_location(Some(test_loc()));
    match filled {
        EvalError::UndefinedVariable { location, .. } => {
            assert_eq!(location, Some(test_loc()));
        }
        other => panic!("variant changed: {other:?}"),
    }

    let kept = EvalError::UndefinedMacro {
        name: "y".to_string(),
        location: Some(SourceLocation {
            file: "inner.md".to_string(),
            line: 9,
            col: 9,
        }),
    }
    .ensure_location(Some(test_loc()));
    match kept {
        EvalError::UndefinedMacro { location, .. } => {
            assert_eq!(location.unwrap().file, "inner.md");
        }
        other => panic!("variant changed: {other:?}"),
    }
}

#[test]
fn ensure_location_passes_through_other_variants_and_none() {
    let io_err = EvalError::IoError(std::io::Error::other("disk"));
    assert!(matches!(
        io_err.ensure_location(Some(test_loc())),
        EvalError::IoError(_)
    ));

    let plain = EvalError::Runtime(None, "r".to_string()).ensure_location(None);
    assert_eq!(plain.to_string(), "Runtime error: r");
}

#[test]
fn parse_error_reports_file() {
    let path = Path::new("broken.md");
    let err = eval_file(path, "%def(unclosed, x, %{no end\n").unwrap_err();

    match &err {
        EvalError::ParseError(location, message) => {
            let loc = location.as_ref().expect("parse error should carry the file");
            assert_eq!(loc.file, "broken.md");
            assert!(!message.is_empty());
            assert!(err.to_string().starts_with("broken.md:"));
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

#[test]
fn node_location_is_none_for_unknown_source() {
    use crate::types::{ASTNode, NodeKind, Token, TokenKind};

    let evaluator = Evaluator::new(EvalConfig::default());
    let node = ASTNode {
        kind: NodeKind::Var,
        src: 99,
        token: Token {
            kind: TokenKind::Text,
            src: 99,
            pos: 0,
            length: 1,
        },
        end_pos: 1,
        parts: Vec::new(),
        name: None,
    };
    assert_eq!(evaluator.node_location(&node), None);
}
