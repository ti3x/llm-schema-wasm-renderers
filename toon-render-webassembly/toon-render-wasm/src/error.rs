use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("TOON conversion failed: {0}")]
    Toon(#[from] toon_wasm::ConvError),

    #[error("render failed: {0}")]
    Render(#[from] json_render_wasm::RenderError),

    #[error("invalid JSON: {0}")]
    Json(String),
}

pub type Result<T> = std::result::Result<T, Error>;
