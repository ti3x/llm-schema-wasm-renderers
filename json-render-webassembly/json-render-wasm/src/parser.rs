//! Walk `serde_json::Value` into the typed `Node` AST. Validates the
//! component tag against the catalog and parses prop values that look
//! like bindings (strings starting with `$`).

use crate::ast::{Node, PropValue};
use crate::catalog::Tag;
use crate::error::{RenderError, RenderResult};
use crate::expr::{looks_like_binding, parse_binding};
use serde_json::Value;
use std::collections::HashMap;

pub fn parse_doc(json: &str) -> RenderResult<Node> {
    let v: Value = serde_json::from_str(json).map_err(|e| RenderError::Json(e.to_string()))?;
    parse_node(&v, "$")
}

fn parse_node(v: &Value, path: &str) -> RenderResult<Node> {
    if let Some(s) = v.as_str() {
        return Ok(Node::Text(s.to_string()));
    }
    let obj = v.as_object().ok_or_else(|| RenderError::Spec {
        path: path.into(),
        msg: "node must be an object with a `tag` field, or a string literal".into(),
    })?;

    let tag_name = obj
        .get("tag")
        .and_then(|t| t.as_str())
        .ok_or_else(|| RenderError::Spec {
            path: path.into(),
            msg: "missing or non-string `tag`".into(),
        })?;
    let tag = Tag::from_name(tag_name, path)?;

    let mut props = HashMap::new();
    if let Some(p) = obj.get("props") {
        let map = p.as_object().ok_or_else(|| RenderError::Spec {
            path: format!("{path}.props"),
            msg: "`props` must be an object".into(),
        })?;
        for (k, v) in map {
            let prop_path = format!("{path}.props.{k}");
            props.insert(k.clone(), parse_prop(v, &prop_path)?);
        }
    }

    let mut children = Vec::new();
    if let Some(c) = obj.get("children") {
        let arr = c.as_array().ok_or_else(|| RenderError::Spec {
            path: format!("{path}.children"),
            msg: "`children` must be an array".into(),
        })?;
        if !arr.is_empty() && !tag.accepts_children() {
            return Err(RenderError::Spec {
                path: path.into(),
                msg: format!("component `{tag_name}` does not accept children"),
            });
        }
        for (i, child) in arr.iter().enumerate() {
            children.push(parse_node(child, &format!("{path}.children[{i}]"))?);
        }
    }

    // Reject unknown keys to keep specs honest and forward-compatible.
    for k in obj.keys() {
        if !matches!(k.as_str(), "tag" | "props" | "children") {
            return Err(RenderError::Spec {
                path: path.into(),
                msg: format!("unknown field `{k}` (allowed: `tag`, `props`, `children`)"),
            });
        }
    }

    Ok(Node::Element { tag, props, children })
}

fn parse_prop(v: &Value, path: &str) -> RenderResult<PropValue> {
    if let Some(s) = v.as_str() {
        if looks_like_binding(s) {
            return parse_binding(s)
                .map(PropValue::Bind)
                .map_err(|e| match e {
                    RenderError::Binding { expr, msg } => RenderError::Spec {
                        path: path.into(),
                        msg: format!("binding `{expr}` rejected: {msg}"),
                    },
                    other => other,
                });
        }
    }
    Ok(PropValue::Lit(v.clone()))
}
