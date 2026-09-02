use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConvError {
    #[error("invalid JSON input: {0}")]
    Json(String),

    #[error("invalid TOON input: {0}")]
    Toon(String),

    #[error("invalid options: {0}")]
    Options(String),
}

pub type ConvResult<T> = Result<T, ConvError>;
