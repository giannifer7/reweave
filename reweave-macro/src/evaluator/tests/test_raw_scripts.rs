// crates/reweave-macro/src/evaluator/tests/test_raw_scripts.rs
//
// Tests for verbatim blocks inside %pydef.
// %[ ... %] and %tag[ ... %tag] keep the script body literal.

use crate::macro_api::process_string_defaults;

// ── %pydef + verbatim blocks ─────────────────────────────────────────────────

#[test]
fn test_pydef_with_verbatim_body_basic() {
    let src = "%pydef(greet, name, %[\"hello \" + name%])%greet(world)";
    let result = process_string_defaults(src).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "hello world");
}

#[test]
fn test_pydef_verbatim_body_not_macro_expanded() {
    let src = "%pydef(check, name, %[\"%(name)\"%])%check(hello)";
    let result = process_string_defaults(src).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "%(name)");
}

#[test]
fn test_pydef_with_tagged_verbatim_body() {
    let src = "%pydef(concat, a, b, %py[a + b%py])%concat(foo, bar)";
    let result = process_string_defaults(src).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "foobar");
}

#[test]
fn test_pydef_macro_aware_body_still_expands() {
    let src = "%pydef(greet, prefix, name, %{\"%(prefix)\"+name%})%greet(hi , world)";
    let result = process_string_defaults(src).unwrap();
    assert_eq!(std::str::from_utf8(&result).unwrap(), "hi world");
}
