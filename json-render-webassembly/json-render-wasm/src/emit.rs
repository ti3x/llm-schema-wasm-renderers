//! Node + state → HTML string, with auto-escaping in text and attribute
//! contexts and URL sanitization on `href` / `src`.

use crate::ast::{Node, PropValue};
use crate::catalog::Tag;
use crate::error::{RenderError, RenderResult};
use crate::expr::Scope;
use serde_json::Value;
use std::collections::HashMap;

/// Stop a hostile or malformed spec from generating multi-MB output.
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
/// Cap total `List` / `Table` row iterations across one render.
const MAX_ITER: usize = 100_000;

pub fn render(root: &Node, state: Value) -> RenderResult<String> {
    let mut scope = Scope::new(state);
    let mut ctx = Ctx {
        out: String::new(),
        iters: 0,
    };
    render_node(root, &mut scope, &mut ctx)?;
    Ok(ctx.out)
}

struct Ctx {
    out: String,
    iters: usize,
}

impl Ctx {
    fn push(&mut self, s: &str) -> RenderResult<()> {
        if self.out.len() + s.len() > MAX_OUTPUT {
            return Err(RenderError::Limit(format!(
                "output exceeded {} byte limit",
                MAX_OUTPUT
            )));
        }
        self.out.push_str(s);
        Ok(())
    }
    fn bump_iter(&mut self) -> RenderResult<()> {
        self.iters += 1;
        if self.iters > MAX_ITER {
            return Err(RenderError::Limit(format!(
                "exceeded {MAX_ITER} loop iterations"
            )));
        }
        Ok(())
    }
}

fn render_node(n: &Node, scope: &mut Scope, ctx: &mut Ctx) -> RenderResult<()> {
    match n {
        Node::Text(s) => ctx.push(&text_escape(s)),
        Node::Element { tag, props, children } => match tag {
            Tag::Text => render_text(props, scope, ctx),
            Tag::Heading => render_heading(props, scope, ctx),
            Tag::Link => render_link(props, scope, ctx),
            Tag::Image => render_image(props, scope, ctx),
            Tag::Container => render_container(children, scope, ctx),
            Tag::Card => render_card(props, children, scope, ctx),
            Tag::Button => render_button(props, scope, ctx),
            Tag::Input => render_input(props, scope, ctx),
            Tag::List => render_list(props, children, scope, ctx),
            Tag::Table => render_table(props, scope, ctx),
        },
    }
}

fn resolve_prop(props: &HashMap<String, PropValue>, key: &str, scope: &Scope) -> Value {
    match props.get(key) {
        Some(PropValue::Lit(v)) => v.clone(),
        Some(PropValue::Bind(b)) => scope.resolve(b),
        None => Value::Null,
    }
}

fn prop_str(props: &HashMap<String, PropValue>, key: &str, scope: &Scope) -> String {
    value_to_string(&resolve_prop(props, key, scope))
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e16 {
                    format!("{}", f as i64)
                } else {
                    f.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

// ─── component renderers ────────────────────────────────────────────────

fn render_text(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let v = prop_str(props, "value", scope);
    ctx.push("<span class=\"jr-text\">")?;
    ctx.push(&text_escape(&v))?;
    ctx.push("</span>")
}

fn render_heading(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let level = resolve_prop(props, "level", scope)
        .as_u64()
        .unwrap_or(2)
        .clamp(1, 6);
    let value = prop_str(props, "value", scope);
    ctx.push(&format!("<h{level} class=\"jr-heading\">"))?;
    ctx.push(&text_escape(&value))?;
    ctx.push(&format!("</h{level}>"))
}

fn render_link(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let href = sanitize_url(&prop_str(props, "href", scope));
    let value = prop_str(props, "value", scope);
    ctx.push("<a class=\"jr-link\" href=\"")?;
    ctx.push(&attr_escape(&href))?;
    ctx.push("\" rel=\"noopener noreferrer\">")?;
    ctx.push(&text_escape(&value))?;
    ctx.push("</a>")
}

fn render_image(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let src = sanitize_url(&prop_str(props, "src", scope));
    let alt = prop_str(props, "alt", scope);
    ctx.push("<img class=\"jr-image\" src=\"")?;
    ctx.push(&attr_escape(&src))?;
    ctx.push("\" alt=\"")?;
    ctx.push(&attr_escape(&alt))?;
    ctx.push("\"/>")
}

fn render_container(children: &[Node], scope: &mut Scope, ctx: &mut Ctx) -> RenderResult<()> {
    ctx.push("<div class=\"jr-container\">")?;
    for c in children {
        render_node(c, scope, ctx)?;
    }
    ctx.push("</div>")
}

fn render_card(
    props: &HashMap<String, PropValue>,
    children: &[Node],
    scope: &mut Scope,
    ctx: &mut Ctx,
) -> RenderResult<()> {
    ctx.push("<div class=\"jr-card\">")?;
    let title = prop_str(props, "title", scope);
    if !title.is_empty() {
        ctx.push("<div class=\"jr-card-title\">")?;
        ctx.push(&text_escape(&title))?;
        ctx.push("</div>")?;
    }
    ctx.push("<div class=\"jr-card-body\">")?;
    for c in children {
        render_node(c, scope, ctx)?;
    }
    ctx.push("</div></div>")
}

fn render_button(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let label = prop_str(props, "label", scope);
    let variant = prop_str(props, "variant", scope);
    let class = if variant.is_empty() {
        "jr-btn".to_string()
    } else if matches!(variant.as_str(), "primary" | "secondary" | "danger" | "ghost") {
        format!("jr-btn jr-btn-{variant}")
    } else {
        // unknown variant — silently fall back to default rather than emit
        // an attacker-controlled class name
        "jr-btn".to_string()
    };
    ctx.push("<button type=\"button\" class=\"")?;
    ctx.push(&attr_escape(&class))?;
    ctx.push("\">")?;
    ctx.push(&text_escape(&label))?;
    ctx.push("</button>")
}

fn render_input(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let placeholder = prop_str(props, "placeholder", scope);
    let value = prop_str(props, "value", scope);
    let raw_type = prop_str(props, "type", scope);
    let typ = if raw_type.is_empty() {
        "text"
    } else {
        match raw_type.as_str() {
            "text" | "email" | "url" | "search" | "tel" | "password" | "number" => {
                raw_type.as_str()
            }
            _ => {
                return Err(RenderError::Spec {
                    path: "Input.props.type".into(),
                    msg: format!("input type `{raw_type}` not allowed"),
                })
            }
        }
    }
    .to_string();
    ctx.push("<input class=\"jr-input\" type=\"")?;
    ctx.push(&attr_escape(&typ))?;
    ctx.push("\" placeholder=\"")?;
    ctx.push(&attr_escape(&placeholder))?;
    ctx.push("\" value=\"")?;
    ctx.push(&attr_escape(&value))?;
    ctx.push("\"/>")
}

fn render_list(
    props: &HashMap<String, PropValue>,
    children: &[Node],
    scope: &mut Scope,
    ctx: &mut Ctx,
) -> RenderResult<()> {
    let items_val = resolve_prop(props, "items", scope);
    let items = items_val.as_array().cloned().unwrap_or_default();
    ctx.push("<ul class=\"jr-list\">")?;
    for (i, item) in items.iter().enumerate() {
        ctx.bump_iter()?;
        scope.item_stack.push((item.clone(), i));
        ctx.push("<li>")?;
        for c in children {
            render_node(c, scope, ctx)?;
        }
        ctx.push("</li>")?;
        scope.item_stack.pop();
    }
    ctx.push("</ul>")
}

fn render_table(props: &HashMap<String, PropValue>, scope: &Scope, ctx: &mut Ctx) -> RenderResult<()> {
    let columns_val = resolve_prop(props, "columns", scope);
    let rows_val = resolve_prop(props, "rows", scope);
    let columns = columns_val.as_array().cloned().unwrap_or_default();
    let rows = rows_val.as_array().cloned().unwrap_or_default();
    ctx.push("<table class=\"jr-table\"><thead><tr>")?;
    for col in &columns {
        let header = col.get("header").and_then(|v| v.as_str()).unwrap_or("");
        ctx.push("<th>")?;
        ctx.push(&text_escape(header))?;
        ctx.push("</th>")?;
    }
    ctx.push("</tr></thead><tbody>")?;
    for row in &rows {
        ctx.bump_iter()?;
        ctx.push("<tr>")?;
        for col in &columns {
            let field = col.get("field").and_then(|v| v.as_str()).unwrap_or("");
            let cell = row.get(field).cloned().unwrap_or(Value::Null);
            ctx.push("<td>")?;
            ctx.push(&text_escape(&value_to_string(&cell)))?;
            ctx.push("</td>")?;
        }
        ctx.push("</tr>")?;
    }
    ctx.push("</tbody></table>")
}

// ─── escaping helpers ──────────────────────────────────────────────────

fn text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Drop dangerous URL schemes. Matches what mainstream renderers do for
/// untrusted href/src: anything starting with `javascript:`, `data:`, or
/// `vbscript:` becomes `#`.
fn sanitize_url(s: &str) -> String {
    let lower = s.trim_start().to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
    {
        "#".into()
    } else {
        s.to_string()
    }
}
