use super::*;

impl Parser {
    fn parse_token_from_parts(parts: Vec<&str>) -> Result<Token, ParserError> {
        if parts.len() != 4 {
            return Err(ParserError::TokenData(format!(
                "Invalid token data: {}",
                parts.join(",")
            )));
        }
        Ok(Token {
            src: parts[0]
                .parse()
                .map_err(|e| ParserError::TokenData(format!("Invalid src: {}", e)))?,
            kind: parts[1]
                .parse::<i32>()
                .map_err(|e| ParserError::TokenData(format!("Invalid kind: {}", e)))?
                .try_into()?,
            pos: parts[2]
                .parse()
                .map_err(|e| ParserError::TokenData(format!("Invalid pos: {}", e)))?,
            length: parts[3]
                .parse()
                .map_err(|e| ParserError::TokenData(format!("Invalid length: {}", e)))?,
        })
    }

    pub(in crate::parser) fn parse_tokens<I>(lines: I) -> Result<Vec<Token>, ParserError>
    where
        I: Iterator<Item = Result<String, std::io::Error>>,
    {
        let mut tokens = Vec::new();
        for line in lines {
            let line =
                line.map_err(|e| ParserError::TokenData(format!("Failed to read line: {}", e)))?;
            let parts: Vec<&str> = line.split(',').collect();
            tokens.push(Self::parse_token_from_parts(parts)?);
        }
        Ok(tokens)
    }

    pub fn read_tokens(path: &str) -> Result<Vec<Token>, ParserError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse_tokens(reader.lines())
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn read_tokens_from_stdin() -> Result<Vec<Token>, ParserError> {
        let stdin = io::stdin();
        Self::parse_tokens(stdin.lock().lines())
    }
}
