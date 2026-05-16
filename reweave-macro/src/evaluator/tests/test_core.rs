// crates/reweave-macro/src/evaluator/tests/test_core.rs
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::evaluator::core::Evaluator;
use crate::evaluator::output::{EvalOutput, PlainOutput};
use crate::evaluator::state::EvalConfig;
use crate::types::{ASTNode, NodeKind, Token, TokenKind};

#[test]
fn test_core_accessors() {
    let config = EvalConfig {
        sigil: '§',
        allow_env: true,
        ..EvalConfig::default()
    };
    let mut eval = Evaluator::new(config);
    assert_eq!(eval.get_sigil(), "§".as_bytes());
    assert!(eval.allow_env());
    assert_eq!(eval.num_source_files(), 0);

    let current = PathBuf::from("/tmp/current.adoc");
    eval.set_current_file(current.clone());
    assert_eq!(eval.get_current_file_path(), current);

    let ast = eval
        .parse_string("hello", &PathBuf::from("<inline>"))
        .unwrap();
    assert_eq!(eval.evaluate(&ast).unwrap(), "hello");

    let mut out = PlainOutput::new();
    eval.evaluate_to(&ast, &mut out).unwrap();
    assert_eq!(out.finish(), "hello");
}

#[test]
fn test_core_store_helpers_round_trip() {
    let mut eval = Evaluator::new(EvalConfig::default());

    eval.pystore_set("py".into(), "world".into());

    assert_eq!(eval.pystore_get("py"), "world");
    assert_eq!(eval.pystore_get("missing"), "");
}

#[test]
fn test_core_record_and_drain_definition_helpers() {
    let mut eval = Evaluator::new(EvalConfig::default());
    eval.record_var_def("answer".into(), 1, 2, 3);
    eval.record_macro_def("greet".into(), 4, 5, 6);

    let var_defs = eval.drain_var_defs();
    let macro_defs = eval.drain_macro_defs();
    assert_eq!(var_defs.len(), 1);
    assert_eq!(var_defs[0].var_name, "answer");
    assert_eq!(macro_defs.len(), 1);
    assert_eq!(macro_defs[0].macro_name, "greet");
    assert!(eval.drain_var_defs().is_empty());
    assert!(eval.drain_macro_defs().is_empty());
}

#[test]
fn test_core_parse_string_real_file_and_source_wrappers() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("sample.adoc");
    fs::write(&path, "content").unwrap();
    let mut eval = Evaluator::new(EvalConfig::default());

    let src1 = eval.add_source_if_not_present(path.clone()).unwrap();
    let src2 = eval.add_source_if_not_present(path.clone()).unwrap();
    assert_eq!(src1, src2);

    let _ = eval.parse_string("content", &path).unwrap();
    assert!(!eval.source_files().is_empty());

    let extra = eval.add_source_bytes(b"inline".to_vec(), PathBuf::from("virt.txt"));
    assert!(extra >= src1);
    assert_eq!(
        eval.sources()
            .source_files()
            .last()
            .unwrap()
            .to_string_lossy(),
        "virt.txt"
    );
}

#[test]
fn test_discover_includes_in_string_records_paths() {
    let temp = TempDir::new().unwrap();
    let include_path = temp.path().join("inc.txt");
    fs::write(&include_path, "included").unwrap();
    let mut eval = Evaluator::new(EvalConfig {
        include_paths: vec![temp.path().to_path_buf()],
        ..EvalConfig::default()
    });

    let discovered =
        crate::macro_api::discover_includes_in_string("%include(inc.txt)", None, &mut eval)
            .unwrap();
    assert_eq!(discovered, vec![include_path]);
    assert!(eval.take_discovered_dependency_paths().is_empty());
}

#[test]
fn test_core_do_include_accepts_absolute_existing_path() {
    let temp = TempDir::new().unwrap();
    let include_path = temp.path().join("abs.txt");
    fs::write(&include_path, "absolute include").unwrap();
    let mut eval = Evaluator::new(EvalConfig::default());

    let result = eval.do_include(include_path.to_str().unwrap()).unwrap();
    assert_eq!(result, "absolute include");
}

#[test]
fn test_node_text_handles_special_tokens_and_invalid_ranges() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let src = eval.add_source_bytes(b"%name(%(var)%".to_vec(), PathBuf::from("inline.txt"));

    let node = |kind, token_kind, pos, length| ASTNode {
        kind,
        src,
        token: Token {
            src,
            kind: token_kind,
            pos,
            length,
        },
        end_pos: pos + length,
        name: None,
        parts: vec![],
    };

    assert_eq!(
        eval.node_text(&node(NodeKind::Macro, TokenKind::Macro, 0, 6)),
        "name"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Var, TokenKind::Var, 6, 6)),
        "var"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Text, TokenKind::Special, 12, 1)),
        "%"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Macro, TokenKind::Macro, 0, 1)),
        "%"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Var, TokenKind::Var, 0, 2)),
        "%n"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Text, TokenKind::Special, 0, 2)),
        "%"
    );
    assert_eq!(
        eval.node_text(&node(NodeKind::Text, TokenKind::Text, 99, 1)),
        ""
    );

    let invalid = ASTNode {
        kind: NodeKind::Text,
        src: 99,
        token: Token {
            src: 99,
            kind: TokenKind::Text,
            pos: 0,
            length: 1,
        },
        end_pos: 1,
        name: None,
        parts: vec![],
    };
    assert_eq!(eval.node_text(&invalid), "");
}

#[test]
fn test_core_default_evaluation_branch_and_temporary_variables() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let src = eval.add_source_bytes(b"%(x)".to_vec(), PathBuf::from("inline.txt"));
    eval.set_variable("x", "outer");

    let var = ASTNode {
        kind: NodeKind::Var,
        src,
        token: Token {
            src,
            kind: TokenKind::Var,
            pos: 0,
            length: 4,
        },
        end_pos: 4,
        name: None,
        parts: vec![],
    };
    let wrapper = ASTNode {
        kind: NodeKind::NotUsed,
        src,
        token: Token::synthetic(src, 0),
        end_pos: 4,
        name: None,
        parts: vec![var],
    };

    let result = eval
        .evaluate_with_temporary_variables(
            &[
                ("x".to_string(), "first".to_string()),
                ("x".to_string(), "second".to_string()),
            ],
            &wrapper,
        )
        .unwrap();
    assert_eq!(result, "second");
    assert_eq!(eval.evaluate(&wrapper).unwrap(), "outer");

    let mut out = PlainOutput::new();
    eval.evaluate_to(&wrapper, &mut out).unwrap();
    assert_eq!(out.finish(), "outer");
}

#[test]
fn test_extract_name_value_returns_empty_for_bad_ranges_and_sources() {
    let mut eval = Evaluator::new(EvalConfig::default());
    let src = eval.add_source_bytes(b"abc".to_vec(), PathBuf::from("inline.txt"));

    assert_eq!(
        eval.extract_name_value(&Token {
            src,
            kind: TokenKind::Ident,
            pos: 1,
            length: 99,
        }),
        ""
    );
    assert_eq!(
        eval.extract_name_value(&Token {
            src: 99,
            kind: TokenKind::Ident,
            pos: 0,
            length: 1,
        }),
        ""
    );
}
