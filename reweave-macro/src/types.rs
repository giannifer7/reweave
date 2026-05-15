use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TokenKind {
    Text = 0,
    Space = 1,
    Special = 2,
    BlockOpen = 3,
    BlockClose = 4,
    Macro = 5,
    Var = 6,
    Ident = 7,
    Comma = 8,
    CloseParen = 9,
    Equal = 10,
    LineComment = 11,
    CommentOpen = 12,
    CommentClose = 13,
    VerbatimOpen = 14,
    VerbatimClose = 15,
    EOF = 16,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Discriminant 0 is intentionally reserved so Rust and Python discriminants align:
    /// Python IntEnum starts at 1 by default, matching Text=1 here.
    NotUsed = 0,
    Text = 1,
    Space = 2,
    Ident = 3,
    LineComment = 4,
    BlockComment = 5,
    Var = 6,
    Equal = 7,
    Param = 8,
    Macro = 9,
    Block = 10,
}
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub src: u32,
    pub pos: usize,
    pub length: usize,
}

impl Token {
    /// One-past-the-end byte offset: `pos + length`.
    pub fn end(&self) -> usize {
        self.pos + self.length
    }

    /// Create a zero-length synthetic token for structural parse nodes that
    /// have no corresponding source token (root block, implicit first param).
    /// `pos` anchors the node in the source for `end_pos` computation.
    pub fn synthetic(src: u32, pos: usize) -> Self {
        Token {
            kind: TokenKind::Text,
            src,
            pos,
            length: 0,
        }
    }
}
#[derive(Debug)]
pub struct LexerError {
    pub pos: usize,
    pub message: String,
}
#[derive(Debug, Clone)]
pub struct ParseNode {
    pub kind: NodeKind,
    pub src: u32,
    pub token: Token,
    pub end_pos: usize,
    pub parts: Vec<usize>,
}
#[derive(Debug, Clone)]
pub struct ASTNode {
    pub kind: NodeKind,
    pub src: u32,
    pub token: Token,
    pub end_pos: usize,
    pub parts: Vec<ASTNode>,
    pub name: Option<Token>,
}
impl TryFrom<i32> for TokenKind {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TokenKind::Text),
            1 => Ok(TokenKind::Space),
            2 => Ok(TokenKind::Special),
            3 => Ok(TokenKind::BlockOpen),
            4 => Ok(TokenKind::BlockClose),
            5 => Ok(TokenKind::Macro),
            6 => Ok(TokenKind::Var),
            7 => Ok(TokenKind::Ident),
            8 => Ok(TokenKind::Comma),
            9 => Ok(TokenKind::CloseParen),
            10 => Ok(TokenKind::Equal),
            11 => Ok(TokenKind::LineComment),
            12 => Ok(TokenKind::CommentOpen),
            13 => Ok(TokenKind::CommentClose),
            14 => Ok(TokenKind::VerbatimOpen),
            15 => Ok(TokenKind::VerbatimClose),
            16 => Ok(TokenKind::EOF),
            _ => Err(format!("Invalid token kind: {}", value)),
        }
    }
}
impl TryFrom<i32> for NodeKind {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NodeKind::NotUsed),
            1 => Ok(NodeKind::Text),
            2 => Ok(NodeKind::Space),
            3 => Ok(NodeKind::Ident),
            4 => Ok(NodeKind::LineComment),
            5 => Ok(NodeKind::BlockComment),
            6 => Ok(NodeKind::Var),
            7 => Ok(NodeKind::Equal),
            8 => Ok(NodeKind::Param),
            9 => Ok(NodeKind::Macro),
            10 => Ok(NodeKind::Block),
            _ => Err(format!("Invalid NodeKind: {value}")),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{NodeKind, Token, TokenKind};
    use std::convert::TryFrom;

    #[test]
    fn token_end_and_synthetic_are_consistent() {
        let token = Token {
            kind: TokenKind::Macro,
            src: 7,
            pos: 11,
            length: 5,
        };
        assert_eq!(token.end(), 16);

        let synthetic = Token::synthetic(9, 23);
        assert_eq!(synthetic.kind, TokenKind::Text);
        assert_eq!(synthetic.src, 9);
        assert_eq!(synthetic.pos, 23);
        assert_eq!(synthetic.length, 0);
        assert_eq!(synthetic.end(), 23);
    }

    #[test]
    fn token_kind_try_from_accepts_full_range() {
        let expected = [
            TokenKind::Text,
            TokenKind::Space,
            TokenKind::Special,
            TokenKind::BlockOpen,
            TokenKind::BlockClose,
            TokenKind::Macro,
            TokenKind::Var,
            TokenKind::Ident,
            TokenKind::Comma,
            TokenKind::CloseParen,
            TokenKind::Equal,
            TokenKind::LineComment,
            TokenKind::CommentOpen,
            TokenKind::CommentClose,
            TokenKind::VerbatimOpen,
            TokenKind::VerbatimClose,
            TokenKind::EOF,
        ];
        for (i, kind) in expected.into_iter().enumerate() {
            assert_eq!(TokenKind::try_from(i as i32).unwrap(), kind);
        }
    }

    #[test]
    fn token_kind_try_from_rejects_invalid_values() {
        assert!(TokenKind::try_from(-1).is_err());
        assert!(TokenKind::try_from(17).is_err());
        assert!(TokenKind::try_from(99).is_err());
    }

    #[test]
    fn node_kind_try_from_accepts_full_range() {
        let expected = [
            NodeKind::NotUsed,
            NodeKind::Text,
            NodeKind::Space,
            NodeKind::Ident,
            NodeKind::LineComment,
            NodeKind::BlockComment,
            NodeKind::Var,
            NodeKind::Equal,
            NodeKind::Param,
            NodeKind::Macro,
            NodeKind::Block,
        ];
        for (i, kind) in expected.into_iter().enumerate() {
            assert_eq!(NodeKind::try_from(i as i32).unwrap(), kind);
        }
    }

    #[test]
    fn node_kind_try_from_rejects_invalid_values() {
        assert!(NodeKind::try_from(-1).is_err());
        assert!(NodeKind::try_from(11).is_err());
        assert!(NodeKind::try_from(99).is_err());
    }
}
