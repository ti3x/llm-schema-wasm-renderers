#![forbid(unsafe_code)]

pub mod ast;
pub mod emit;
pub mod error;
pub mod expr;
pub mod lexer;
pub mod parser;

pub use error::{PugError, PugResult};

/// Compile a pug source string + JSON locals to an HTML string.
pub fn render(source: &str, data_json: &str) -> PugResult<String> {
    let data: serde_json::Value = if data_json.trim().is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_json::from_str(data_json).map_err(|e| PugError::Json(e.to_string()))?
    };
    let tokens = lexer::lex(source)?;
    let doc = parser::parse(&tokens)?;
    emit::render(&doc, &data)
}

#[cfg(feature = "wasm")]
mod wasm {
    use crate::{ast::Doc, emit, lexer, parser, PugError};
    use serde_json::Value;
    use wasm_bindgen::prelude::*;

    /// One-shot: `compile(source, locals_json) -> html` or throws.
    /// Convenient for templates rendered once.
    #[wasm_bindgen]
    pub fn compile(source: &str, locals_json: &str) -> Result<String, JsError> {
        super::render(source, locals_json).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Parse a pug source once and render it many times against different
    /// locals. Avoids re-lexing / re-parsing when the same backend-supplied
    /// template is reused with different data.
    ///
    /// Always call `.free()` (or use `using` / similar) — wasm-bindgen
    /// doesn't garbage-collect Rust-side resources for you.
    #[wasm_bindgen]
    pub struct Template {
        doc: Doc,
    }

    #[wasm_bindgen]
    impl Template {
        #[wasm_bindgen(constructor)]
        pub fn new(source: &str) -> Result<Template, JsError> {
            let tokens = lexer::lex(source).map_err(|e| JsError::new(&e.to_string()))?;
            let doc = parser::parse(&tokens).map_err(|e| JsError::new(&e.to_string()))?;
            Ok(Template { doc })
        }

        pub fn render(&self, locals_json: &str) -> Result<String, JsError> {
            let data: Value = if locals_json.trim().is_empty() {
                Value::Object(Default::default())
            } else {
                serde_json::from_str(locals_json)
                    .map_err(|e| JsError::new(&PugError::Json(e.to_string()).to_string()))?
            };
            emit::render(&self.doc, &data).map_err(|e| JsError::new(&e.to_string()))
        }
    }
}
