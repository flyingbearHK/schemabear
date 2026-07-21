use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("validation error: {0}")]
    Validation(String),

    #[error("unknown entity referenced: {0}")]
    UnknownEntity(String),

    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }
}
