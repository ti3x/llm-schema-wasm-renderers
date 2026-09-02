//! Port of `toon-webassembly/web/combo.js` `adapt()`.
//!
//! Converts the json-render.dev indirected spec form
//!
//! ```json
//! { "root": "id", "state": { .. }, "elements": { "id": { "type", "props", "children": [id..] } } }
//! ```
//!
//! with `$state` / `$template` bindings — into the renderer's flat
//! `{ tag, props, children }` form, resolving all bindings against the
//! embedded `state`. A spec already in the flat form passes through
//! unchanged. The output only ever uses catalog tags (`Text`, `Heading`,
//! `Container`, `Card`, `Button`) so it is always accepted by
//! `json-render-wasm`; components outside that set are synthesized from
//! primitives, matching the original JS demo.

use serde_json::{json, Map, Value};

/// Adapt a decoded spec into the renderer's flat form.
pub fn adapt(spec: Value) -> Value {
    if !is_indirected(&spec) {
        return spec; // already flat form
    }
    let state = spec.get("state").cloned().unwrap_or_else(|| json!({}));
    let empty = Map::new();
    let elements = spec
        .get("elements")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let root = spec.get("root").and_then(Value::as_str).unwrap_or("");
    build_by_id(root, elements, &state)
}

fn is_indirected(spec: &Value) -> bool {
    spec.get("root").is_some_and(Value::is_string)
        && spec.get("elements").is_some_and(Value::is_object)
}

fn build_by_id(id: &str, elements: &Map<String, Value>, state: &Value) -> Value {
    match elements.get(id) {
        Some(node) => build_node(node, elements, state),
        None => text(&format!("[missing element: {id}]")),
    }
}

fn build_node(node: &Value, elements: &Map<String, Value>, state: &Value) -> Value {
    // Resolve bindings inside every prop.
    let mut props = Map::new();
    if let Some(raw) = node.get("props").and_then(Value::as_object) {
        for (k, v) in raw {
            props.insert(k.clone(), resolve_bindings(v, state));
        }
    }

    let children: Vec<Value> = node
        .get("children")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(|id| build_by_id(id, elements, state))
                .collect()
        })
        .unwrap_or_default();

    let node_type = node.get("type").and_then(Value::as_str).unwrap_or("");

    match node_type {
        "Card" => {
            let mut p = Map::new();
            if let Some(title) = props.get("title").filter(|v| !v.is_null()) {
                p.insert("title".into(), Value::String(to_str(title)));
            }
            json!({ "tag": "Card", "props": Value::Object(p), "children": children })
        }
        "Container" | "Stack" => json!({ "tag": "Container", "children": children }),
        "Heading" => json!({
            "tag": "Heading",
            "props": {
                "level": level_from(props.get("level")),
                "value": text_of(&props, &["text", "value"]),
            }
        }),
        "Text" => json!({
            "tag": "Text",
            "props": { "value": text_of(&props, &["text", "value"]) }
        }),
        "Button" => json!({
            "tag": "Button",
            "props": {
                "label": text_of(&props, &["label"]),
                "variant": text_of(&props, &["variant"]),
            }
        }),
        "Image" => {
            // The preview iframe's CSP blocks remote images — emit a styled
            // Container with alt text and dimensions instead of an <img>.
            let alt = opt_str(&props, "alt").unwrap_or_else(|| "Image".into());
            let w = opt_str(&props, "width").unwrap_or_else(|| "—".into());
            let h = opt_str(&props, "height").unwrap_or_else(|| "—".into());
            json!({
                "tag": "Container",
                "children": [
                    heading(5, &format!("🖼 {alt}")),
                    text(&format!("{w} × {h}")),
                ]
            })
        }
        "Separator" => json!({
            "tag": "Container",
            "children": [ text("──────────") ]
        }),
        "Metric" => {
            let prefix = opt_str(&props, "prefix").unwrap_or_default();
            let value = opt_str(&props, "value").unwrap_or_default();
            let change = opt_str(&props, "change").unwrap_or_default();
            let label = opt_str(&props, "label").unwrap_or_default();
            let mut kids = Vec::new();
            if !label.is_empty() {
                kids.push(text(&label));
            }
            kids.push(heading(3, &format!("{prefix}{value}")));
            if !change.is_empty() {
                kids.push(text(&change));
            }
            json!({ "tag": "Container", "children": kids })
        }
        "LineGraph" => {
            let values = number_series(props.get("data"));
            let summary = if values.is_empty() {
                String::new()
            } else {
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                format!(
                    "{} points · min {} · max {}",
                    values.len(),
                    num(min),
                    num(max)
                )
            };
            let spark = sparkline(&values);
            json!({
                "tag": "Container",
                "children": [
                    heading(4, "LineGraph"),
                    text(if spark.is_empty() { "(no data)" } else { &spark }),
                    text(&summary),
                ]
            })
        }
        "Progress" => {
            let label = opt_str(&props, "label").unwrap_or_default();
            let v = props.get("value").and_then(as_f64).unwrap_or(0.0);
            let mut kids = Vec::new();
            if !label.is_empty() {
                kids.push(text(&label));
            }
            kids.push(text(&progress_bar(v, 20)));
            json!({ "tag": "Container", "children": kids })
        }
        // Unknown component — render as a labeled placeholder so the page
        // still loads, keeping any children.
        other => {
            let mut kids = vec![heading(5, &format!("[{other}]"))];
            kids.extend(children);
            json!({ "tag": "Container", "children": kids })
        }
    }
}

// ─── binding resolution ──────────────────────────────────────────────────

fn resolve_bindings(value: &Value, state: &Value) -> Value {
    match value {
        Value::Array(a) => Value::Array(a.iter().map(|v| resolve_bindings(v, state)).collect()),
        Value::Object(o) => {
            if let Some(Value::String(ptr)) = o.get("$state") {
                return json_pointer(state, ptr).cloned().unwrap_or(Value::Null);
            }
            if let Some(Value::String(tmpl)) = o.get("$template") {
                return Value::String(interpolate(tmpl, state));
            }
            let mut out = Map::new();
            for (k, v) in o {
                out.insert(k.clone(), resolve_bindings(v, state));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// JSON-pointer lookup matching combo.js `jsonPointer`: tolerates a
/// missing leading `/`, decodes `~1`/`~0`, indexes arrays by number.
fn json_pointer<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() || path == "/" {
        return Some(root);
    }
    let path = path.strip_prefix('/').unwrap_or(path);
    let mut cur = root;
    for seg in path.split('/') {
        let key = decode_pointer(seg);
        cur = match cur {
            Value::Object(m) => m.get(&key)?,
            Value::Array(a) => a.get(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn decode_pointer(s: &str) -> String {
    s.replace("~1", "/").replace("~0", "~")
}

/// Replace every `${/pointer}` in `tmpl` with the resolved value.
fn interpolate(tmpl: &str, state: &Value) -> String {
    let mut out = String::new();
    let mut rest = tmpl;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        match rest.find('}') {
            Some(end) => {
                let ptr = &rest[..end];
                match json_pointer(state, ptr) {
                    Some(v) if !v.is_null() => out.push_str(&to_str(v)),
                    _ => {}
                }
                rest = &rest[end + 1..];
            }
            None => {
                out.push_str("${");
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

// ─── small helpers (ported from combo.js) ────────────────────────────────

fn text(value: &str) -> Value {
    json!({ "tag": "Text", "props": { "value": value } })
}

fn heading(level: u64, value: &str) -> Value {
    json!({ "tag": "Heading", "props": { "level": level, "value": value } })
}

/// `String(props.a ?? props.b ?? "")` over already-resolved props.
fn text_of(props: &Map<String, Value>, keys: &[&str]) -> String {
    for k in keys {
        if let Some(v) = props.get(*k).filter(|v| !v.is_null()) {
            return to_str(v);
        }
    }
    String::new()
}

/// `props.k != null ? String(props.k) : None`.
fn opt_str(props: &Map<String, Value>, key: &str) -> Option<String> {
    props.get(key).filter(|v| !v.is_null()).map(to_str)
}

/// JS `String(value)` for the value kinds we encounter.
fn to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// `levelFromString`: parse leading int of `String(s)` sans a leading `h`,
/// clamp to 1..=6, default 2.
fn level_from(v: Option<&Value>) -> u64 {
    let s = v.map(to_str).unwrap_or_else(|| "2".into());
    let trimmed = s.trim_start_matches(['h', 'H']);
    let digits: String = trimmed
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    match digits.parse::<u64>() {
        Ok(n) if (1..=6).contains(&n) => n,
        _ => 2,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// `data` may be an array of numbers or of `{ value }` objects.
fn number_series(data: Option<&Value>) -> Vec<f64> {
    match data.and_then(Value::as_array) {
        Some(arr) => arr
            .iter()
            .map(|p| {
                p.get("value")
                    .and_then(as_f64)
                    .or_else(|| as_f64(p))
                    .unwrap_or(0.0)
            })
            .collect(),
        None => Vec::new(),
    }
}

fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let span = if max - min == 0.0 { 1.0 } else { max - min };
    values
        .iter()
        .map(|v| {
            let idx = (((v - min) / span) * (bars.len() - 1) as f64).floor() as usize;
            bars[idx.min(bars.len() - 1)]
        })
        .collect()
}

fn progress_bar(pct: f64, width: usize) -> String {
    let v = pct.clamp(0.0, 100.0);
    let filled = ((v / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        "[{}{}] {}%",
        "█".repeat(filled),
        "░".repeat(width - filled),
        num(v)
    )
}

/// Format a number without a trailing `.0`, like JS string coercion.
fn num(f: f64) -> String {
    if f.fract() == 0.0 && f.is_finite() {
        format!("{}", f as i64)
    } else {
        f.to_string()
    }
}
