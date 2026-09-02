//! End-to-end smoke tests. Each builds a JSON spec, encodes it to TOON via
//! the reused `toon-wasm` crate, then renders it through the full pipeline —
//! exercising toon→json→adapt→html for both spec forms.

use serde_json::{json, Value};
use toon_wasm::options::ConvOptions;

fn to_toon(spec: &Value) -> String {
    toon_wasm::json_to_toon(&spec.to_string(), &ConvOptions::default()).expect("encode TOON")
}

#[test]
fn native_flat_form() {
    let spec = json!({
        "tag": "Card",
        "props": { "title": "Greeting" },
        "children": [ { "tag": "Text", "props": { "value": "hello world" } } ]
    });
    let html = toon_render_wasm::render(&to_toon(&spec), "").expect("render");
    assert!(html.contains("Greeting"), "title missing: {html}");
    assert!(html.contains("hello world"), "text missing: {html}");
    assert!(html.contains("jr-card"), "card markup missing: {html}");
}

#[test]
fn indirected_form_with_bindings() {
    let spec = json!({
        "root": "card",
        "state": { "user": "Ada", "count": 42 },
        "elements": {
            "card": { "type": "Card", "props": { "title": "Dashboard" }, "children": ["h", "t"] },
            "h": { "type": "Heading", "props": { "level": 2, "text": { "$template": "Hi ${/user}" } } },
            "t": { "type": "Text", "props": { "text": { "$state": "/count" } } }
        }
    });
    let html = toon_render_wasm::render(&to_toon(&spec), "").expect("render");
    assert!(html.contains("Dashboard"), "card title missing: {html}");
    assert!(html.contains("Hi Ada"), "template binding unresolved: {html}");
    assert!(html.contains("<h2"), "heading level wrong: {html}");
    assert!(html.contains("42"), "state binding unresolved: {html}");
}

#[test]
fn unknown_component_becomes_placeholder() {
    let spec = json!({
        "root": "x",
        "elements": { "x": { "type": "Widget", "props": {}, "children": [] } }
    });
    let html = toon_render_wasm::render(&to_toon(&spec), "").expect("render");
    assert!(html.contains("[Widget]"), "placeholder missing: {html}");
}

#[test]
fn synthesized_metric_from_primitives() {
    let spec = json!({
        "root": "m",
        "elements": {
            "m": { "type": "Metric", "props": { "label": "Revenue", "prefix": "$", "value": "45.2", "change": "+12%" } }
        }
    });
    let html = toon_render_wasm::render(&to_toon(&spec), "").expect("render");
    assert!(html.contains("Revenue"), "metric label missing: {html}");
    assert!(html.contains("$45.2"), "metric value missing: {html}");
    assert!(html.contains("+12%"), "metric change missing: {html}");
}
