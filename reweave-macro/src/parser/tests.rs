mod basic;
mod invariants;
mod lex_errors;
mod tagged_errors;
mod tagged_valid;

use crate::lexer::Lexer;
use crate::line_index::LineIndex;
use crate::parser::{
    BlockDelim, ParseContext, Parser, ParserState, block_delim_chars, block_tag_label,
};
use crate::types::{NodeKind, Token, TokenKind};
use std::io::Write;

fn lex_parse(src: &str) -> Result<(), String> {
    let (tokens, lex_errors) = Lexer::new(src, '%', 0).lex();
    assert!(
        lex_errors.is_empty(),
        "unexpected lex errors: {:?}",
        lex_errors
    );
    let line_index = LineIndex::new(src);
    let mut parser = Parser::new();
    parser
        .parse(&tokens, src.as_bytes(), &line_index)
        .map_err(|e| e.to_string())
}

fn lex_parse_err(src: &str) -> String {
    let (tokens, lex_errors) = Lexer::new(src, '%', 0).lex();
    if !lex_errors.is_empty() {
        return lex_errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
    }
    let line_index = LineIndex::new(src);
    let mut parser = Parser::new();
    parser
        .parse(&tokens, src.as_bytes(), &line_index)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
}

#[test]
fn read_tokens_from_file_accepts_csv_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokens.csv");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "0,0,0,5").unwrap();
    writeln!(file, "0,16,5,0").unwrap();

    let tokens = Parser::read_tokens(path.to_str().unwrap()).unwrap();

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Text);
    assert_eq!(tokens[0].src, 0);
    assert_eq!(tokens[0].pos, 0);
    assert_eq!(tokens[0].length, 5);
    assert_eq!(tokens[1].kind, TokenKind::EOF);
}

#[test]
fn read_tokens_rejects_malformed_csv_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.csv");
    std::fs::write(&path, "0,0,0\n").unwrap();

    let err = Parser::read_tokens(path.to_str().unwrap()).unwrap_err();

    assert!(err.to_string().contains("Invalid token data"));
}

#[test]
fn read_tokens_rejects_invalid_numeric_fields() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_src = dir.path().join("invalid-src.csv");
    let invalid_kind = dir.path().join("invalid-kind.csv");
    let invalid_pos = dir.path().join("invalid-pos.csv");
    let invalid_length = dir.path().join("invalid-length.csv");
    std::fs::write(&invalid_src, "x,0,0,1\n").unwrap();
    std::fs::write(&invalid_kind, "0,99,0,1\n").unwrap();
    std::fs::write(&invalid_pos, "0,0,x,1\n").unwrap();
    std::fs::write(&invalid_length, "0,0,0,x\n").unwrap();

    assert!(
        Parser::read_tokens(invalid_src.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("Invalid src")
    );
    assert!(
        Parser::read_tokens(invalid_kind.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("Invalid token kind")
    );
    assert!(
        Parser::read_tokens(invalid_pos.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("Invalid pos")
    );
    assert!(
        Parser::read_tokens(invalid_length.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("Invalid length")
    );
}

#[test]
fn read_tokens_reports_line_read_errors() {
    let err =
        Parser::parse_tokens(vec![Err(std::io::Error::other("line read failed"))].into_iter())
            .unwrap_err();

    assert!(err.to_string().contains("Failed to read line"));
}

#[test]
fn parser_accessors_and_ast_builders_cover_empty_and_valid_trees() {
    let mut empty = Parser::new();
    assert_eq!(empty.get_root_index(), None);
    assert!(
        empty
            .process_ast(b"")
            .unwrap_err()
            .contains("Empty parse tree")
    );
    let default_parser = Parser::default();
    assert_eq!(default_parser.get_root_index(), None);

    let src = "hello ";
    let (tokens, errors) = Lexer::new(src, '%', 0).lex();
    assert!(errors.is_empty());
    let line_index = LineIndex::new(src);
    let mut parser = Parser::new();
    parser.parse(&tokens, src.as_bytes(), &line_index).unwrap();

    let root = parser.get_root_index().unwrap();
    assert!(parser.get_node(root).is_some());
    assert_eq!(
        parser.get_node_info(root).unwrap().1,
        crate::types::NodeKind::Block
    );
    assert!(!parser.to_json().is_empty());
    assert!(parser.build_ast().is_ok());
    assert!(parser.process_ast(src.as_bytes()).is_ok());
}

#[test]
fn parser_private_formatting_helpers_cover_edge_cases() {
    let line_index = LineIndex::new("abc");
    let ctx = ParseContext::new(b"abc", &line_index);
    assert!(ctx.tags_match((0, 0), (2, 0)));
    assert!(!ctx.tags_match((0, 1), (1, 2)));
    assert!(!ctx.tags_match((99, 1), (0, 1)));
    let bad_utf8_index = LineIndex::new("x");
    let bad_utf8 = [0xff];
    let bad_utf8_ctx = ParseContext::new(&bad_utf8, &bad_utf8_index);
    assert_eq!(bad_utf8_ctx.tag_str(0, 1), "");
    assert_eq!(block_tag_label("", '{'), "(anonymous)");
    assert_eq!(block_tag_label("name", '}'), "%name}");
    assert_eq!(block_delim_chars(BlockDelim::Curly), ('{', '}'));
    assert_eq!(block_delim_chars(BlockDelim::Square), ('[', ']'));
}

#[test]
fn strip_ending_space_handles_missing_and_out_of_bounds_nodes() {
    let mut parser = Parser::new();
    assert!(
        parser
            .strip_ending_space(b"abc", 99)
            .unwrap_err()
            .contains("not found")
    );

    let idx = parser.add_node(crate::types::ParseNode {
        kind: crate::types::NodeKind::Text,
        src: 0,
        token: Token {
            kind: TokenKind::Text,
            src: 0,
            pos: 10,
            length: 3,
        },
        end_pos: 13,
        parts: Vec::new(),
    });
    parser.strip_ending_space(b"abc", idx).unwrap();
    assert_eq!(parser.get_node(idx).unwrap().token.length, 3);

    let idx = parser.add_node(crate::types::ParseNode {
        kind: crate::types::NodeKind::Text,
        src: 0,
        token: Token {
            kind: TokenKind::Text,
            src: 0,
            pos: 0,
            length: 4,
        },
        end_pos: 4,
        parts: Vec::new(),
    });
    parser.strip_ending_space(b"ab \n", idx).unwrap();
    assert_eq!(parser.get_node(idx).unwrap().token.length, 2);
}

fn token(kind: TokenKind, pos: usize, length: usize) -> Token {
    Token {
        kind,
        src: 0,
        pos,
        length,
    }
}

#[test]
fn parser_accepts_empty_token_stream() {
    let mut parser = Parser::new();
    let line_index = LineIndex::new("");
    parser.parse(&[], b"", &line_index).unwrap();
    assert_eq!(parser.get_root_index(), None);
}

#[test]
fn parser_reports_internal_stack_invariant_errors() {
    let mut parser = Parser::new();
    parser.stack.push((
        ParserState::Block {
            tag_pos: 0,
            tag_len: 0,
            delim: BlockDelim::Curly,
        },
        999,
    ));
    let err = parser.close_top(1).unwrap_err();
    assert!(err.to_string().contains("not in arena"));
}

#[test]
fn parser_reports_empty_stack_after_malformed_root_close() {
    let source = "%}text";
    let tokens = vec![
        token(TokenKind::BlockClose, 0, 2),
        token(TokenKind::Text, 2, 4),
    ];
    let line_index = LineIndex::new(source);
    let mut parser = Parser::new();

    let err = parser
        .parse(&tokens, source.as_bytes(), &line_index)
        .unwrap_err();

    assert!(err.to_string().contains("empty parser stack"));
}

#[test]
fn parser_unwind_stack_clears_empty_stack_without_work() {
    let mut parser = Parser::new();
    parser.unwind_stack(0);
    assert_eq!(parser.stack.len(), 0);
}

#[test]
fn parser_reports_unclosed_structures_from_token_stream() {
    for (source, tokens, expected) in [
        (
            "%foo(",
            vec![token(TokenKind::Macro, 0, 5), token(TokenKind::EOF, 5, 0)],
            "unclosed macro argument list",
        ),
        (
            "%/*",
            vec![
                token(TokenKind::CommentOpen, 0, 3),
                token(TokenKind::EOF, 3, 0),
            ],
            "unclosed block comment",
        ),
        (
            "%{",
            vec![
                token(TokenKind::BlockOpen, 0, 2),
                token(TokenKind::EOF, 2, 0),
            ],
            "unclosed block",
        ),
    ] {
        let mut parser = Parser::new();
        let line_index = LineIndex::new(source);
        let err = parser
            .parse(&tokens, source.as_bytes(), &line_index)
            .unwrap_err();
        assert!(
            err.to_string().contains(expected),
            "expected {expected:?} in {err}"
        );
    }
}

#[test]
fn parser_direct_handlers_cover_internal_error_and_nested_comments() {
    let source = "%/* nested %*/ %*/";
    let line_index = LineIndex::new(source);
    let ctx = ParseContext::new(source.as_bytes(), &line_index);
    let mut parser = Parser::new();

    let root = parser.add_node(crate::types::ParseNode {
        kind: NodeKind::Block,
        src: 0,
        token: Token::synthetic(0, 0),
        end_pos: 0,
        parts: vec![],
    });
    parser.stack.push((
        ParserState::Block {
            tag_pos: 0,
            tag_len: 0,
            delim: BlockDelim::Curly,
        },
        root,
    ));
    parser.stack.push((ParserState::Param, root));
    let err = parser
        .handle_param(token(TokenKind::CloseParen, 0, 1))
        .unwrap_err();
    assert!(err.to_string().contains("expected Macro below Param"));

    let comment = parser.add_node(crate::types::ParseNode {
        kind: NodeKind::BlockComment,
        src: 0,
        token: token(TokenKind::CommentOpen, 0, 3),
        end_pos: 3,
        parts: vec![],
    });
    parser.stack.clear();
    parser.stack.push((ParserState::Comment, comment));
    parser
        .handle_comment(token(TokenKind::CommentOpen, 4, 3))
        .unwrap();
    assert!(matches!(
        parser.stack.last().map(|(state, _)| *state),
        Some(ParserState::Comment)
    ));
    parser
        .handle_comment(token(TokenKind::CommentClose, 12, 3))
        .unwrap();
    assert!(matches!(
        parser.stack.last().map(|(state, _)| *state),
        Some(ParserState::Comment)
    ));

    let mut block_parser = Parser::new();
    let block = block_parser.add_node(crate::types::ParseNode {
        kind: NodeKind::Block,
        src: 0,
        token: token(TokenKind::BlockOpen, 0, 2),
        end_pos: 2,
        parts: vec![],
    });
    block_parser.stack.push((
        ParserState::Block {
            tag_pos: 0,
            tag_len: 0,
            delim: BlockDelim::Square,
        },
        block,
    ));
    assert!(
        !block_parser
            .handle_block(
                token(TokenKind::BlockClose, 0, 2),
                &ctx,
                0,
                0,
                BlockDelim::Square
            )
            .unwrap()
    );

    let invalid_tag = Token {
        kind: TokenKind::BlockOpen,
        src: 0,
        pos: 0,
        length: 2,
    };
    assert_eq!(Parser::block_tag(&invalid_tag, &[0xff]), (1, 0));
}
