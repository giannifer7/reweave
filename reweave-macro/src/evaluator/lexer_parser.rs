// reweave/crates/reweave-macro/src/evaluator/lexer_parser.rs

use crate::lexer::Lexer;
use crate::line_index::LineIndex;
use crate::parser::Parser;
use crate::types::ASTNode;

pub fn lex_parse_content(source: &str, sigil: char, src: u32) -> Result<ASTNode, String> {
    let (tokens, lex_errors) = Lexer::new(source, sigil, src).lex();
    let line_index = LineIndex::new(source);
    if !lex_errors.is_empty() {
        let errs = lex_errors
            .iter()
            .map(|e| {
                let (line, col) = line_index.line_col(e.pos);
                format!("{}:{}: {}", line, col, e.message)
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("Lexer errors: {}", errs));
    }

    let mut parser = Parser::new();
    parser
        .parse(&tokens, source.as_bytes(), &line_index)
        .map_err(|e| format!("Parse error: {}", e))?;

    let ast = parser
        .process_ast(source.as_bytes())
        .map_err(|e| format!("AST build error: {:?}", e))?;

    Ok(ast)
}
