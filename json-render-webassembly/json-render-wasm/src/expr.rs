//! Restricted binding-expression parser + resolver.
//!
//! Security model: the only legal binding forms are
//!
//!   $bindState.<path>     read from the render-time state
//!   $item                 the current list item (inside a `List`)
//!   $item.<path>          a field of the current list item
//!   $index                the current zero-based index inside a `List`
//!
//! Path segments are ASCII identifier characters only (`[A-Za-z0-9_]`)
//! or non-negative integers (for array indexing). Anything that looks
//! like code — parentheses, operators, JS keywords, reserved property
//! names — is rejected at parse time. There is no eval; the resolver is
//! a pure walk over `serde_json::Value`.

use crate::error::{RenderError, RenderResult};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum Binding {
    /// `$bindState.<path>`
    State(Vec<String>),
    /// `$item` (empty path) or `$item.<path>`
    Item(Vec<String>),
    /// `$index`
    Index,
}

/// True only for strings that exactly match one of the three binding
/// roots, optionally followed by a `.<path>`. This deliberately rejects
/// strings like `"$12.99"` or `"$249.00"` that happen to start with `$`
/// but are obviously literal text — the parser should not try to
/// interpret them as bindings.
pub fn looks_like_binding(s: &str) -> bool {
    s == "$bindState"
        || s == "$item"
        || s == "$index"
        || s.starts_with("$bindState.")
        || s.starts_with("$item.")
}

const BANNED_TOKENS: &[&str] = &[
    "eval",
    "function",
    "this",
    "new",
    "delete",
    "typeof",
    "instanceof",
    "void",
    "yield",
    "await",
    "throw",
    "arguments",
    "constructor",
    "__proto__",
    "prototype",
];

pub fn parse_binding(src: &str) -> RenderResult<Binding> {
    // Fast rejects on suspicious characters. Bindings only ever contain
    // `$`, ASCII identifier chars, digits, and `.` as a path separator.
    for c in src.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
        if !ok {
            return Err(RenderError::Binding {
                expr: src.into(),
                msg: format!("disallowed character `{c}`"),
            });
        }
    }
    for kw in BANNED_TOKENS {
        if src.contains(kw) {
            return Err(RenderError::Binding {
                expr: src.into(),
                msg: format!("disallowed token `{kw}`"),
            });
        }
    }
    let mut parts = src.split('.');
    let head = parts.next().unwrap_or("");
    let rest: Vec<String> = parts.map(|s| s.to_string()).collect();
    for seg in &rest {
        validate_segment(seg, src)?;
    }
    match head {
        "$bindState" => Ok(Binding::State(rest)),
        "$item" => Ok(Binding::Item(rest)),
        "$index" => {
            if !rest.is_empty() {
                return Err(RenderError::Binding {
                    expr: src.into(),
                    msg: "`$index` does not take a path".into(),
                });
            }
            Ok(Binding::Index)
        }
        _ => Err(RenderError::Binding {
            expr: src.into(),
            msg: format!(
                "unknown binding root `{head}` (allowed: `$bindState`, `$item`, `$index`)"
            ),
        }),
    }
}

fn validate_segment(seg: &str, full: &str) -> RenderResult<()> {
    if seg.is_empty() {
        return Err(RenderError::Binding {
            expr: full.into(),
            msg: "empty path segment".into(),
        });
    }
    // Belt-and-braces — these are caught by BANNED_TOKENS too, but be loud.
    if matches!(seg, "constructor" | "__proto__" | "prototype") {
        return Err(RenderError::Binding {
            expr: full.into(),
            msg: format!("access to `{seg}` is not allowed"),
        });
    }
    if !seg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RenderError::Binding {
            expr: full.into(),
            msg: format!("invalid path segment `{seg}`"),
        });
    }
    Ok(())
}

/// Render-time binding scope. Read-only; mutation happens via the
/// item-stack which is pushed/popped by `List` iteration.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub state: Value,
    pub item_stack: Vec<(Value, usize)>,
}

impl Scope {
    pub fn new(state: Value) -> Self {
        Self {
            state,
            item_stack: Vec::new(),
        }
    }

    pub fn resolve(&self, b: &Binding) -> Value {
        match b {
            Binding::State(path) => walk(&self.state, path),
            Binding::Item(path) => {
                let item = self
                    .item_stack
                    .last()
                    .map(|(v, _)| v.clone())
                    .unwrap_or(Value::Null);
                walk(&item, path)
            }
            Binding::Index => {
                let idx = self.item_stack.last().map(|(_, i)| *i).unwrap_or(0);
                Value::Number(serde_json::Number::from(idx))
            }
        }
    }
}

fn walk(v: &Value, path: &[String]) -> Value {
    let mut cur = v.clone();
    for seg in path {
        cur = match cur {
            Value::Object(mut m) => m.remove(seg).unwrap_or(Value::Null),
            Value::Array(arr) => {
                if let Ok(i) = seg.parse::<usize>() {
                    arr.get(i).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        };
    }
    cur
}
