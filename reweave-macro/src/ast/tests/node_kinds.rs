use super::*;

#[test]
fn test_node_kind_discriminants() {
    // NotUsed=0 is intentional: Python IntEnum starts at 1 by default,
    // so reserving 0 keeps Rust and Python discriminants aligned.
    assert_eq!(NodeKind::NotUsed as i32, 0);
    assert_eq!(NodeKind::Text as i32, 1);
    assert_eq!(NodeKind::Space as i32, 2);
    assert_eq!(NodeKind::Ident as i32, 3);
    assert_eq!(NodeKind::LineComment as i32, 4);
    assert_eq!(NodeKind::BlockComment as i32, 5);
    assert_eq!(NodeKind::Var as i32, 6);
    assert_eq!(NodeKind::Equal as i32, 7);
    assert_eq!(NodeKind::Param as i32, 8);
    assert_eq!(NodeKind::Macro as i32, 9);
    assert_eq!(NodeKind::Block as i32, 10);
}
