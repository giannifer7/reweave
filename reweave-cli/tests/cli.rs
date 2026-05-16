use std::process::Command;

#[test]
fn cli_binary_expands_markdown_to_files() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.md");
    let out = temp.path().join("out");
    std::fs::write(
        &input,
        "```text\n# <[@file result.txt]>=\n%def(word, hello)%word()\n# @\n```",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_reweave"))
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read_to_string(out.join("result.txt")).unwrap(),
        "hello\n"
    );
}

#[test]
fn cli_binary_macro_only_writes_expanded_markdown_to_stdout() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.md");
    let out = temp.path().join("out");
    std::fs::write(
        &input,
        "before %def(word, hello)%word()\n```text\n# <[@file result.txt]>=\nbody\n# @\n```",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_reweave"))
        .arg(&input)
        .arg("--macro-only")
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "before hello\n```text\n# <[@file result.txt]>=\nbody\n# @\n```"
    );
    assert!(!out.join("result.txt").exists());
}

#[test]
fn cli_binary_rejects_macro_only_with_no_macro() {
    let output = Command::new(env!("CARGO_BIN_EXE_reweave"))
        .arg("--macro-only")
        .arg("--no-macro")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("cannot be used with")
    );
}
