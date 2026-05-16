// crates/reweave-macro/src/ast/serialization.rs — generated from ast.adoc
use crate::evaluator::{EvalError, lex_parse_content};
use crate::types::{ASTNode, Token};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn serialize_token(token: &Token) -> String {
    format!(
        "{},{},{},{}",
        token.src, token.kind as i32, token.pos, token.length
    )
}

pub fn serialize_ast_nodes(root: &ASTNode) -> Vec<String> {
    let mut nodes = Vec::new();
    // BFS so that child indices assigned as next_idx..next_idx+n are contiguous
    // and land exactly where each node ends up in the output array.
    let mut queue: VecDeque<&ASTNode> = VecDeque::new();
    let mut next_idx = 1usize; // root is index 0

    // We don't need to write src because we process one file at a time and the caller knows which
    queue.push_back(root);
    while let Some(node) = queue.pop_front() {
        let child_indices: Vec<usize> = (next_idx..next_idx + node.parts.len()).collect();
        next_idx += node.parts.len();

        let parts = if child_indices.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                child_indices
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        nodes.push(format!(
            "[{},{},{},{}]",
            node.kind as i32,
            serialize_token(&node.token),
            node.end_pos,
            parts,
        ));

        for child in &node.parts {
            queue.push_back(child);
        }
    }

    nodes
}

pub fn write_ast<W: Write>(header: &str, nodes: &[String], writer: &mut W) -> io::Result<()> {
    writeln!(writer, "{}", header)?;
    for line in nodes {
        writeln!(writer, "{}", line)?;
    }
    Ok(())
}

pub fn write_ast_to_file(header: &str, nodes: &[String], output_path: &PathBuf) -> io::Result<()> {
    if output_path.to_str() == Some("-") {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        write_ast(header, nodes, &mut handle)
    } else {
        let mut file = File::create(output_path)?;
        write_ast(header, nodes, &mut file)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn read_input(input: &PathBuf) -> io::Result<String> {
    if input.to_str() == Some("-") {
        read_stdin()
    } else {
        std::fs::read_to_string(input)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn read_stdin() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

pub fn dump_macro_ast(sigil: char, input_files: &[PathBuf]) -> Result<(), EvalError> {
    for input in input_files {
        let content = read_input(input).map_err(|e| {
            EvalError::Runtime(format!("Failed to read {}: {}", input.display(), e))
        })?;

        let ast = lex_parse_content(&content, sigil, 0)?;
        let nodes = serialize_ast_nodes(&ast);

        let (output, src_name) = ast_output_for_input(input);

        // Header line: maps src indices to source file paths.
        // Format: # src:<index>=<path>  (one per source file; currently always src:0)
        let header = format!("# src:0={}", src_name);

        write_ast_to_file(&header, &nodes, &output).map_err(|e| {
            EvalError::Runtime(format!("Failed to write {}: {}", output.display(), e))
        })?;
    }
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn ast_output_for_input(input: &Path) -> (PathBuf, String) {
    if input.to_str() == Some("-") {
        (PathBuf::from("-"), "-".to_string())
    } else {
        (input.with_extension("ast"), input.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ASTNode, NodeKind, Token, TokenKind};
    use tempfile::tempdir;

    fn token(kind: TokenKind, pos: usize, length: usize) -> Token {
        Token {
            src: 0,
            kind,
            pos,
            length,
        }
    }

    fn sample_ast() -> ASTNode {
        ASTNode {
            kind: NodeKind::Block,
            src: 0,
            token: Token::synthetic(0, 0),
            end_pos: 7,
            name: None,
            parts: vec![
                ASTNode {
                    kind: NodeKind::Text,
                    src: 0,
                    token: token(TokenKind::Text, 0, 3),
                    end_pos: 3,
                    name: None,
                    parts: vec![],
                },
                ASTNode {
                    kind: NodeKind::Macro,
                    src: 0,
                    token: token(TokenKind::Macro, 3, 4),
                    end_pos: 7,
                    name: None,
                    parts: vec![ASTNode {
                        kind: NodeKind::Param,
                        src: 0,
                        token: token(TokenKind::Ident, 4, 2),
                        end_pos: 6,
                        name: Some(token(TokenKind::Ident, 4, 1)),
                        parts: vec![],
                    }],
                },
            ],
        }
    }

    #[test]
    fn serialize_ast_nodes_emits_breadth_first_indices() {
        let lines = serialize_ast_nodes(&sample_ast());
        assert_eq!(lines.len(), 4);
        assert!(lines[0].ends_with("[1,2]]"));
        assert!(lines[1].ends_with("[]]"));
        assert!(lines[2].ends_with("[3]]"));
        assert!(lines[3].ends_with("[]]"));
    }

    #[test]
    fn write_ast_and_write_ast_to_file_emit_expected_content() {
        let nodes = vec!["[10,0,0,0,[]]".to_string(), "[1,0,0,3,[]]".to_string()];
        let mut out = Vec::new();
        write_ast("# src:0=input", &nodes, &mut out).expect("write ast");
        let text = String::from_utf8(out).expect("utf8");
        assert_eq!(text, "# src:0=input\n[10,0,0,0,[]]\n[1,0,0,3,[]]\n");

        let dir = tempdir().expect("tempdir");
        let output = dir.path().join("sample.ast");
        write_ast_to_file("# src:0=input", &nodes, &output).expect("write file");
        assert_eq!(std::fs::read_to_string(output).expect("read file"), text);

        write_ast_to_file("# src:0=-", &nodes, &PathBuf::from("-")).expect("write stdout");
    }

    #[test]
    fn dump_macro_ast_writes_ast_file_next_to_input() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("sample.txt");
        std::fs::write(&input, "hello %name(world)").expect("write input");

        dump_macro_ast('%', std::slice::from_ref(&input)).expect("dump ast");

        let output = input.with_extension("ast");
        let text = std::fs::read_to_string(output).expect("read ast");
        assert!(text.starts_with("# src:0="));
        assert!(text.lines().count() > 1);
    }

    #[test]
    fn dump_macro_ast_reports_input_and_parse_errors() {
        let dir = tempdir().expect("tempdir");
        let missing = dir.path().join("missing.txt");
        let err = dump_macro_ast('%', &[missing]).unwrap_err();
        assert!(err.to_string().contains("Failed to read"));

        let invalid = dir.path().join("invalid.txt");
        std::fs::write(&invalid, "%foo(").expect("write invalid input");
        let err = dump_macro_ast('%', &[invalid]).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn dump_macro_ast_reports_output_write_errors() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("sample.txt");
        let blocked_output = dir.path().join("sample.ast");
        std::fs::write(&input, "plain").expect("write input");
        std::fs::create_dir(&blocked_output).expect("create blocking directory");

        let err = dump_macro_ast('%', &[input]).unwrap_err();

        assert!(err.to_string().contains("Failed to write"));
    }
}
