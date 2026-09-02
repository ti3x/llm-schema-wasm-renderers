use thiserror::Error;

#[derive(Debug, Error)]
pub enum PugError {
    #[error("lex error at line {line}, col {col}: {msg}")]
    Lex { line: usize, col: usize, msg: String },

    #[error("parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },

    #[error("expression error at line {line}: {msg}")]
    Expr { line: usize, msg: String },

    #[error("evaluation error: {0}")]
    Eval(String),

    #[error("invalid JSON locals: {0}")]
    Json(String),

    #[error("rendering aborted: {0}")]
    Limit(String),
}

pub type PugResult<T> = Result<T, PugError>;
