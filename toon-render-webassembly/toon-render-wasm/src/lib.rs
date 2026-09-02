#![forbid(unsafe_code)]

//! Single-module `TOON → JSON UI spec → HTML` renderer.
//!
//! An LLM emits a UI spec as [TOON](https://github.com/toon-format/toon)
//! (a token-efficient JSON encoding); this crate decodes it to JSON,
//! adapts the json-render.dev indirected form to the renderer's flat
//! form (resolving `$state`/`$template` bindings), and renders safe HTML.
//!
//! It is pure glue over two sibling crates — [`toon_wasm`] does TOON⇄JSON
//! and [`json_render_wasm`] does spec→HTML — plus the [`adapt`] module,
//! which ports the JS-side adapter from `toon-webassembly`'s combo demo.

pub mod adapt;
pub mod error;

pub use error::{Error, Result};

use serde_json::Value;
use toon_wasm::options::ConvOptions;

/// TOON spec → HTML, resolving any embedded state during adaptation.
///
/// `state_json` is forwarded to the renderer for native-form
/// `$bindState` bindings in specs that were already flat; indirected
/// json-render.dev specs resolve their `$state`/`$template` bindings
/// against their own embedded `state` before this call.
pub fn render(toon_spec: &str, state_json: &str) -> Result<String> {
    let spec = adapt_toon(toon_spec)?;
    let spec_json = serde_json::to_string(&spec).map_err(|e| Error::Json(e.to_string()))?;
    Ok(json_render_wasm::render(&spec_json, state_json)?)
}

/// Decode TOON to JSON and adapt it to the renderer's flat spec form.
pub fn adapt_toon(toon_spec: &str) -> Result<Value> {
    let json = toon_wasm::toon_to_json(toon_spec, &ConvOptions::default())?;
    let value: Value = serde_json::from_str(&json).map_err(|e| Error::Json(e.to_string()))?;
    Ok(adapt::adapt(value))
}

#[cfg(feature = "wasm")]
mod wasm {
    use serde_json::Value;
    use wasm_bindgen::prelude::*;

    /// One-shot: `compile(toon_spec, state) -> html` or throws.
    #[wasm_bindgen]
    pub fn compile(toon_spec: &str, state_json: &str) -> Result<String, JsError> {
        super::render(toon_spec, state_json).map_err(|e| JsError::new(&e.to_string()))
    }

    /// The JSON a TOON spec decodes to (pretty-printed) — the 1:1 form of
    /// the same data, before adaptation. Lets a UI show the TOON⇄JSON diff.
    #[wasm_bindgen]
    pub fn to_json(toon_spec: &str) -> Result<String, JsError> {
        use toon_wasm::options::ConvOptions;
        toon_wasm::toon_to_json(toon_spec, &ConvOptions::default())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Parse + adapt a TOON spec once, then render it against different
    /// state values. Bindings embedded in an indirected spec are resolved
    /// at construction; `render(state)` supplies state for any residual
    /// native-form `$bindState` bindings.
    #[wasm_bindgen]
    pub struct Template {
        spec: Value,
    }

    #[wasm_bindgen]
    impl Template {
        #[wasm_bindgen(constructor)]
        pub fn new(toon_spec: &str) -> Result<Template, JsError> {
            let spec = super::adapt_toon(toon_spec).map_err(|e| JsError::new(&e.to_string()))?;
            Ok(Template { spec })
        }

        pub fn render(&self, state_json: &str) -> Result<String, JsError> {
            let spec_json =
                serde_json::to_string(&self.spec).map_err(|e| JsError::new(&e.to_string()))?;
            json_render_wasm::render(&spec_json, state_json)
                .map_err(|e| JsError::new(&e.to_string()))
        }
    }
}
