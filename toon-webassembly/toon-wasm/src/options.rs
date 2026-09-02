//! Conversion options shared by both directions. Deserialized from a
//! JSON object on the JS side so the playground can pass a single
//! second argument to either WASM export.

use serde::Deserialize;
use toon_format::types::{KeyFoldingMode, PathExpansionMode};
use toon_format::{DecodeOptions, Delimiter as ToonDelim, EncodeOptions, Indent};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConvOptions {
    pub delimiter: Delimiter,
    pub indent: u8,
    /// Decoder strict mode (rejects mismatched array lengths, bad indent, …).
    pub strict: bool,
    /// Decoder coerces `"42"` → `42`, `"true"` → `true`, etc.
    pub coerce_types: bool,
    /// Encoder folds single-key object chains into dotted paths.
    pub key_folding: bool,
    /// Decoder expands dotted keys into nested objects.
    pub expand_paths: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Delimiter {
    #[default]
    Comma,
    Tab,
    Pipe,
}

impl Default for ConvOptions {
    fn default() -> Self {
        Self {
            delimiter: Delimiter::Comma,
            indent: 2,
            strict: true,
            coerce_types: true,
            key_folding: false,
            expand_paths: false,
        }
    }
}

impl Delimiter {
    fn to_toon(self) -> ToonDelim {
        match self {
            Delimiter::Comma => ToonDelim::Comma,
            Delimiter::Tab => ToonDelim::Tab,
            Delimiter::Pipe => ToonDelim::Pipe,
        }
    }
}

impl ConvOptions {
    pub fn to_encode(&self) -> EncodeOptions {
        let folding = if self.key_folding {
            KeyFoldingMode::Safe
        } else {
            KeyFoldingMode::Off
        };
        EncodeOptions::new()
            .with_delimiter(self.delimiter.to_toon())
            .with_spaces(self.indent as usize)
            .with_key_folding(folding)
    }

    pub fn to_decode(&self) -> DecodeOptions {
        let expand = if self.expand_paths {
            PathExpansionMode::Safe
        } else {
            PathExpansionMode::Off
        };
        DecodeOptions::new()
            .with_strict(self.strict)
            .with_coerce_types(self.coerce_types)
            .with_delimiter(self.delimiter.to_toon())
            .with_indent(Indent::Spaces(self.indent as usize))
            .with_expand_paths(expand)
    }
}
