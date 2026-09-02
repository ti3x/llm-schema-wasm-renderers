#![forbid(unsafe_code)]

pub mod error;
pub mod options;

pub use error::{ConvError, ConvResult};
use options::ConvOptions;
use serde_json::Value;

/// JSON string → TOON string with the given options.
pub fn json_to_toon(json: &str, opts: &ConvOptions) -> ConvResult<String> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| ConvError::Json(e.to_string()))?;
    toon_format::encode(&value, &opts.to_encode()).map_err(|e| ConvError::Toon(e.to_string()))
}

/// TOON string → pretty-printed JSON string with the given options.
pub fn toon_to_json(toon: &str, opts: &ConvOptions) -> ConvResult<String> {
    let value: Value = toon_format::decode(toon, &opts.to_decode())
        .map_err(|e| ConvError::Toon(e.to_string()))?;
    serde_json::to_string_pretty(&value).map_err(|e| ConvError::Json(e.to_string()))
}

#[cfg(feature = "wasm")]
mod wasm {
    use crate::options::ConvOptions;
    use wasm_bindgen::prelude::*;

    fn parse_opts(opts_json: &str) -> Result<ConvOptions, JsError> {
        if opts_json.trim().is_empty() {
            return Ok(ConvOptions::default());
        }
        serde_json::from_str(opts_json).map_err(|e| JsError::new(&e.to_string()))
    }

    /// JSON → TOON. `opts_json` accepts an empty string for defaults, or
    /// a JSON object with any subset of `{ delimiter, indent, strict,
    /// coerceTypes, keyFolding, expandPaths }`.
    #[wasm_bindgen]
    pub fn json_to_toon(json: &str, opts_json: &str) -> Result<String, JsError> {
        let opts = parse_opts(opts_json)?;
        super::json_to_toon(json, &opts).map_err(|e| JsError::new(&e.to_string()))
    }

    /// TOON → JSON (pretty-printed, 2-space indent).
    #[wasm_bindgen]
    pub fn toon_to_json(toon: &str, opts_json: &str) -> Result<String, JsError> {
        let opts = parse_opts(opts_json)?;
        super::toon_to_json(toon, &opts).map_err(|e| JsError::new(&e.to_string()))
    }
}
