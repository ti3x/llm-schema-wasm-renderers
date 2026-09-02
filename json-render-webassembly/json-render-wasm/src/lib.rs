#![forbid(unsafe_code)]

pub mod ast;
pub mod catalog;
pub mod emit;
pub mod error;
pub mod expr;
pub mod parser;

pub use error::{RenderError, RenderResult};

/// Render a JSON UI spec against a JSON state document, returning HTML.
pub fn render(spec_json: &str, state_json: &str) -> RenderResult<String> {
    let state: serde_json::Value = if state_json.trim().is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_json::from_str(state_json).map_err(|e| RenderError::Json(e.to_string()))?
    };
    let root = parser::parse_doc(spec_json)?;
    emit::render(&root, state)
}

#[cfg(feature = "wasm")]
mod wasm {
    use crate::{ast::Node, emit, parser, RenderError};
    use serde_json::Value;
    use wasm_bindgen::prelude::*;

    /// One-shot: `compile(spec, state) -> html` or throws. Use when the
    /// spec changes every render anyway.
    #[wasm_bindgen]
    pub fn compile(spec_json: &str, state_json: &str) -> Result<String, JsError> {
        super::render(spec_json, state_json).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Parse a spec once and render it many times against different
    /// state values. Avoids re-parsing JSON + revalidating the catalog
    /// on each frame. Call `.free()` to release the Rust-side AST.
    #[wasm_bindgen]
    pub struct Template {
        root: Node,
    }

    #[wasm_bindgen]
    impl Template {
        #[wasm_bindgen(constructor)]
        pub fn new(spec_json: &str) -> Result<Template, JsError> {
            let root = parser::parse_doc(spec_json).map_err(|e| JsError::new(&e.to_string()))?;
            Ok(Template { root })
        }

        pub fn render(&self, state_json: &str) -> Result<String, JsError> {
            let state: Value = if state_json.trim().is_empty() {
                Value::Object(Default::default())
            } else {
                serde_json::from_str(state_json).map_err(|e| {
                    JsError::new(&RenderError::Json(e.to_string()).to_string())
                })?
            };
            emit::render(&self.root, state).map_err(|e| JsError::new(&e.to_string()))
        }
    }
}
