// crates/reweave-macro/src/evaluator/tests/test_state.rs
use std::collections::HashMap;
use std::sync::Arc;

use crate::evaluator::output::{SourceSpan, SpanKind, SpanRange};
use crate::evaluator::state::{
    EvalConfig, EvaluatorState, MacroBindingKind, MacroDefinition, ScriptKind, SourceManager,
};
use crate::types::{ASTNode, NodeKind, Token, TokenKind};
use tempfile::TempDir;

fn dummy_ast() -> Arc<ASTNode> {
    Arc::new(ASTNode {
        kind: NodeKind::Block,
        src: 0,
        token: Token {
            src: 0,
            kind: TokenKind::Text,
            pos: 0,
            length: 0,
        },
        end_pos: 0,
        parts: vec![],
        name: None,
    })
}

#[test]
fn test_source_manager_add_source_if_not_present_deduplicates() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.txt");
    std::fs::write(&path, b"hello").unwrap();

    let mut sm = SourceManager::new();
    let first = sm.add_source_if_not_present(path.clone()).unwrap();
    let second = sm.add_source_if_not_present(path).unwrap();

    assert_eq!(first, second);
    assert_eq!(sm.num_sources(), 1);
    assert_eq!(sm.get_source(first).unwrap(), b"hello");
}

#[test]
fn test_source_manager_add_source_bytes_and_files_list() {
    let mut sm = SourceManager::new();
    let src = sm.add_source_bytes(b"abc".to_vec(), "virt.txt".into());
    assert_eq!(src, 0);
    assert_eq!(sm.num_sources(), 1);
    assert_eq!(sm.source_files()[0].to_string_lossy(), "virt.txt");
    assert_eq!(sm.get_source(0).unwrap(), b"abc");
    assert!(sm.get_source(9).is_none());
}

#[test]
fn test_state_variables_are_current_frame_only() {
    let mut st = EvaluatorState::new(EvalConfig::default());
    st.set_variable("x", "outer");
    assert_eq!(st.get_variable("x"), "outer");

    st.push_scope();
    assert_eq!(st.get_variable_opt("x"), None);
    assert_eq!(st.get_variable("x"), "");
    st.set_variable("x", "inner");
    assert_eq!(st.get_variable("x"), "inner");

    st.pop_scope();
    assert_eq!(st.get_variable("x"), "outer");

    st.pop_scope();
    assert_eq!(st.get_variable("x"), "outer");
    assert_eq!(st.scope_stack.len(), 1);
}

#[test]
fn test_state_tracked_and_traced_variables() {
    let mut st = EvaluatorState::new(EvalConfig::default());
    let span = SourceSpan {
        src: 0,
        pos: 3,
        length: 2,
        kind: SpanKind::Literal,
    };
    st.set_tracked_variable("a", "hi", Some(span.clone()));
    let tracked = st.get_tracked_variable("a").unwrap();
    assert_eq!(tracked.value, "hi");
    assert_eq!(tracked.spans.len(), 1);
    assert_eq!(tracked.spans[0].start, 0);
    assert_eq!(tracked.spans[0].end, 2);

    st.set_traced_variable(
        "b",
        "hello".to_string(),
        vec![SpanRange {
            start: 1,
            end: 4,
            span,
        }],
    );
    let traced = st.get_tracked_variable("b").unwrap();
    assert_eq!(traced.value, "hello");
    assert_eq!(traced.spans.len(), 1);
    assert_eq!(traced.spans[0].start, 1);
    assert_eq!(traced.spans[0].end, 4);

    st.set_tracked_variable("empty_span", "value", None);
    let unspanned = st.get_tracked_variable("empty_span").unwrap();
    assert_eq!(unspanned.value, "value");
    assert!(unspanned.spans.is_empty());
}

#[test]
fn test_state_define_macro_and_lookup_shadowing() {
    let mut st = EvaluatorState::new(EvalConfig::default());
    st.define_macro(MacroDefinition {
        name: "m".into(),
        params: vec!["x".into()],
        body: dummy_ast(),
        script_kind: ScriptKind::None,
        binding_kind: MacroBindingKind::Constant,
        frozen_args: HashMap::new(),
    })
    .unwrap();
    assert_eq!(st.get_macro("m").unwrap().params, vec!["x"]);

    st.push_scope();
    st.define_macro(MacroDefinition {
        name: "m".into(),
        params: vec!["y".into()],
        body: dummy_ast(),
        script_kind: ScriptKind::Python,
        binding_kind: MacroBindingKind::Constant,
        frozen_args: HashMap::new(),
    })
    .unwrap();
    let inner = st.get_macro("m").unwrap();
    assert_eq!(inner.params, vec!["y"]);
    assert_eq!(inner.script_kind, ScriptKind::Python);
    assert_eq!(inner.binding_kind, MacroBindingKind::Constant);

    st.pop_scope();
    let outer = st.get_macro("m").unwrap();
    assert_eq!(outer.params, vec!["x"]);
    assert_eq!(outer.script_kind, ScriptKind::None);
}

#[test]
fn test_state_rejects_same_frame_constant_redefinition() {
    let mut st = EvaluatorState::new(EvalConfig::default());
    st.define_macro(MacroDefinition {
        name: "m".into(),
        params: vec![],
        body: dummy_ast(),
        script_kind: ScriptKind::None,
        binding_kind: MacroBindingKind::Constant,
        frozen_args: HashMap::new(),
    })
    .unwrap();

    let err = st
        .define_macro(MacroDefinition {
            name: "m".into(),
            params: vec!["x".into()],
            body: dummy_ast(),
            script_kind: ScriptKind::None,
            binding_kind: MacroBindingKind::Constant,
            frozen_args: HashMap::new(),
        })
        .unwrap_err();

    assert!(err.to_string().contains("constant binding"));
}

#[test]
fn test_state_rebindable_macro_can_be_replaced() {
    let mut st = EvaluatorState::new(EvalConfig::default());
    st.redefine_macro(MacroDefinition {
        name: "m".into(),
        params: vec![],
        body: dummy_ast(),
        script_kind: ScriptKind::None,
        binding_kind: MacroBindingKind::Rebindable,
        frozen_args: HashMap::new(),
    })
    .unwrap();

    st.redefine_macro(MacroDefinition {
        name: "m".into(),
        params: vec!["x".into()],
        body: dummy_ast(),
        script_kind: ScriptKind::Python,
        binding_kind: MacroBindingKind::Rebindable,
        frozen_args: HashMap::new(),
    })
    .unwrap();

    let mac = st.get_macro("m").unwrap();
    assert_eq!(mac.params, vec!["x"]);
    assert_eq!(mac.script_kind, ScriptKind::Python);
    assert_eq!(mac.binding_kind, MacroBindingKind::Rebindable);
}

#[test]
fn test_state_drain_defs_and_unicode_sigil() {
    let mut st = EvaluatorState::new(EvalConfig {
        sigil: '§',
        ..EvalConfig::default()
    });
    st.var_defs.push(crate::evaluator::state::VarDefRaw {
        var_name: "x".into(),
        src: 0,
        pos: 1,
        length: 2,
    });
    st.macro_defs.push(crate::evaluator::state::MacroDefRaw {
        macro_name: "m".into(),
        src: 0,
        pos: 3,
        length: 4,
    });

    assert_eq!(st.get_sigil(), "§".as_bytes());
    assert_eq!(st.drain_var_defs().len(), 1);
    assert_eq!(st.drain_macro_defs().len(), 1);
    assert!(st.drain_var_defs().is_empty());
    assert!(st.drain_macro_defs().is_empty());
}
