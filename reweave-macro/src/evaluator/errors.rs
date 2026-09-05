use std::fmt;

use thiserror::Error;

/// A 1-indexed source position: `file:line:col`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.col)
    }
}

fn loc_prefix(loc: &Option<SourceLocation>) -> String {
    loc.as_ref().map_or_else(String::new, |l| format!("{l}: "))
}

#[derive(Error, Debug)]
pub enum EvalError {
    #[error("{}Undefined macro: {name}", loc_prefix(.location))]
    UndefinedMacro {
        name: String,
        location: Option<SourceLocation>,
    },

    #[error("{}Undefined variable: {name}", loc_prefix(.location))]
    UndefinedVariable {
        name: String,
        location: Option<SourceLocation>,
    },

    #[error("{}Unbound parameter '{param_name}' in macro '{macro_name}'", loc_prefix(.location))]
    UnboundParameter {
        macro_name: String,
        param_name: String,
        location: Option<SourceLocation>,
    },

    #[error("{}Builtin error: {}", loc_prefix(.0), .1)]
    BuiltinError(Option<SourceLocation>, String),

    #[error("{}Include not found: {}", loc_prefix(.0), .1)]
    IncludeNotFound(Option<SourceLocation>, String),

    #[error("{}Circular include: {}", loc_prefix(.0), .1)]
    CircularInclude(Option<SourceLocation>, String),

    #[error("{}Invalid usage: {}", loc_prefix(.0), .1)]
    InvalidUsage(Option<SourceLocation>, String),

    #[error("{}Runtime error: {}", loc_prefix(.0), .1)]
    Runtime(Option<SourceLocation>, String),

    #[error("{}Parse error: {}", loc_prefix(.0), .1)]
    ParseError(Option<SourceLocation>, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type EvalResult<T> = Result<T, EvalError>;

impl EvalError {
    /// Attach a location to an error that does not already carry one.
    /// The variant is preserved; the location is filled in where the variant
    /// has a slot for it, otherwise it is prepended to the message text.
    /// Errors that already have a location pass through unchanged.
    pub fn ensure_location(self, loc: Option<SourceLocation>) -> EvalError {
        let Some(loc) = loc else { return self };
        match self {
            EvalError::UndefinedVariable { name, location } => EvalError::UndefinedVariable {
                name,
                location: location.or(Some(loc)),
            },
            EvalError::UndefinedMacro { name, location } => EvalError::UndefinedMacro {
                name,
                location: location.or(Some(loc)),
            },
            EvalError::UnboundParameter {
                macro_name,
                param_name,
                location,
            } => EvalError::UnboundParameter {
                macro_name,
                param_name,
                location: location.or(Some(loc)),
            },
            EvalError::BuiltinError(slot, m) => EvalError::BuiltinError(slot.or(Some(loc)), m),
            EvalError::InvalidUsage(slot, m) => EvalError::InvalidUsage(slot.or(Some(loc)), m),
            EvalError::Runtime(slot, m) => EvalError::Runtime(slot.or(Some(loc)), m),
            EvalError::IncludeNotFound(slot, m) => EvalError::IncludeNotFound(slot.or(Some(loc)), m),
            EvalError::CircularInclude(slot, m) => EvalError::CircularInclude(slot.or(Some(loc)), m),
            EvalError::ParseError(slot, m) => EvalError::ParseError(slot.or(Some(loc)), m),
            other => other,
        }
    }
}

impl From<String> for EvalError {
    fn from(s: String) -> Self {
        EvalError::Runtime(None, s)
    }
}
