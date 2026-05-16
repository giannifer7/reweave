// crates/reweave-macro/src/parser/mod.rs — generated from parser.adoc
use crate::line_index::LineIndex;
use crate::types::{ASTNode, NodeKind, ParseNode, Token, TokenKind};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use thiserror::Error;

#[cfg(test)]
mod tests;

/// The parser-specific error type.
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid token data: {0}")]
    TokenData(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<String> for ParserError {
    fn from(s: String) -> Self {
        ParserError::TokenData(s)
    }
}
#[cfg(test)]
impl Token {
    pub fn to_json(&self) -> String {
        format!(
            "[{},{},{},{}]",
            self.src, self.kind as i32, self.pos, self.length
        )
    }
}

#[cfg(test)]
impl ParseNode {
    pub fn to_json(&self) -> String {
        format!(
            "[{},{},{},{},{}]",
            self.kind as i32,
            self.src,
            self.token.to_json(),
            self.end_pos,
            if self.parts.is_empty() {
                "[]".to_string()
            } else {
                format!(
                    "[{}]",
                    self.parts
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        )
    }
}

/// Stack frame state.  Each variant owns its termination tokens;
/// everything else falls through to the shared opener/leaf handler.
///
/// `tag_len == 0` means an anonymous block (`%{`/`%}`).
///
/// `Macro` and `Param` are always paired: `Macro` sits below `Param` on the
/// stack.  `Param` receives individual tokens; `Macro` is only ever visible
/// after `Param` is popped (at `)`) so that `handle_param` can verify the
/// expected stack shape.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockDelim {
    Curly,
    Square,
}

#[derive(Debug, Clone, Copy)]
enum ParserState {
    Block {
        tag_pos: usize,
        tag_len: usize,
        delim: BlockDelim,
    },
    Macro,
    Param,
    Comment,
}

/// Bundles the source bytes with a borrowed `LineIndex`.
/// The index is built once by the caller (who may already need it for lexer
/// error formatting) and passed in, so the O(n) newline scan never happens
/// more than once per source string.
struct ParseContext<'a> {
    content: &'a [u8],
    line_index: &'a LineIndex,
}

impl<'a> ParseContext<'a> {
    fn new(content: &'a [u8], line_index: &'a LineIndex) -> Self {
        Self {
            content,
            line_index,
        }
    }

    fn line_col(&self, pos: usize) -> (usize, usize) {
        self.line_index.line_col(pos)
    }

    /// Compare two tag spans. Anonymous (len == 0) matches anonymous only.
    /// Returns `false` — not a panic — if either span is out of bounds.
    fn tags_match(&self, (ap, al): (usize, usize), (bp, bl): (usize, usize)) -> bool {
        if al != bl {
            return false;
        }
        if al == 0 {
            return true;
        }
        if ap + al > self.content.len() || bp + bl > self.content.len() {
            return false;
        }
        self.content[ap..ap + al] == self.content[bp..bp + bl]
    }

    /// Return the tag as a `&str` for error messages.
    fn tag_str(&self, pos: usize, len: usize) -> &str {
        if len == 0 {
            return "";
        }
        debug_assert!(pos + len <= self.content.len(), "tag span OOB");
        std::str::from_utf8(self.content.get(pos..pos + len).unwrap_or(&[])).unwrap_or("")
    }
}

/// Format a block tag for error messages.
/// Anonymous blocks (`tag == ""`) render as `(anonymous)`;
/// named blocks render as `%name{` / `%name}` or `%name[` / `%name]`.
fn block_tag_label(tag: &str, brace: char) -> String {
    if tag.is_empty() {
        "(anonymous)".to_string()
    } else {
        format!("%{tag}{brace}")
    }
}

fn block_delim_chars(delim: BlockDelim) -> (char, char) {
    match delim {
        BlockDelim::Curly => ('{', '}'),
        BlockDelim::Square => ('[', ']'),
    }
}

pub struct Parser {
    nodes: Vec<ParseNode>,
    stack: Vec<(ParserState, usize)>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

mod api;
mod arena;
mod handlers;
mod parse;
mod token_io;
