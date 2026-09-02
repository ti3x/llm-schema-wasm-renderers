use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("invalid spec JSON: {0}")]
    Json(String),

    #[error("at {path}: {msg}")]
    Spec { path: String, msg: String },

    #[error("binding `{expr}` rejected: {msg}")]
    Binding { expr: String, msg: String },

    #[error("rendering aborted: {0}")]
    Limit(String),
}

pub type RenderResult<T> = Result<T, RenderError>;
