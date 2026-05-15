use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvalError {
    #[error("Undefined macro: {0}")]
    UndefinedMacro(String),

    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    #[error("Unbound parameter '{param_name}' in macro '{macro_name}'")]
    UnboundParameter {
        macro_name: String,
        param_name: String,
    },

    #[error("Builtin error: {0}")]
    BuiltinError(String),

    #[error("Include not found: {0}")]
    IncludeNotFound(String),

    #[error("Circular include: {0}")]
    CircularInclude(String),

    #[error("Invalid usage: {0}")]
    InvalidUsage(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type EvalResult<T> = Result<T, EvalError>;

impl From<String> for EvalError {
    fn from(s: String) -> Self {
        EvalError::Runtime(s)
    }
}
