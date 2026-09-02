//! AST → HTML string, with auto-escaping in text and attribute contexts.

use crate::ast::*;
use crate::error::{PugError, PugResult};
use crate::expr::{self, Scope};
use serde_json::Value;

/// Hard cap on iterations across all `each` loops in a single render —
/// stops a hostile template from blowing up output size.
const MAX_ITER: usize = 100_000;

/// Hard cap on output bytes.
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

pub fn render(doc: &Doc, locals: &Value) -> PugResult<String> {
    let mut scope = Scope::new();
    scope.seed_from(locals);
    let mut ctx = RenderCtx {
        out: String::new(),
        iters: 0,
    };
    render_nodes(&doc.nodes, &mut scope, &mut ctx)?;
    Ok(ctx.out)
}

struct RenderCtx {
    out: String,
    iters: usize,
}

impl RenderCtx {
    fn push(&mut self, s: &str) -> PugResult<()> {
        if self.out.len() + s.len() > MAX_OUTPUT {
            return Err(PugError::Limit("output exceeded 4MB limit".into()));
        }
        self.out.push_str(s);
        Ok(())
    }
    fn bump_iter(&mut self) -> PugResult<()> {
        self.iters += 1;
        if self.iters > MAX_ITER {
            return Err(PugError::Limit(format!(
                "exceeded {MAX_ITER} loop iterations"
            )));
        }
        Ok(())
    }
}

fn render_nodes(nodes: &[Node], scope: &mut Scope, ctx: &mut RenderCtx) -> PugResult<()> {
    for n in nodes {
        render_node(n, scope, ctx)?;
    }
    Ok(())
}

fn render_node(node: &Node, scope: &mut Scope, ctx: &mut RenderCtx) -> PugResult<()> {
    match node {
        Node::Doctype(d) => {
            let s = doctype_str(d.trim());
            ctx.push(&s)?;
        }
        Node::Tag(t) => render_tag(t, scope, ctx)?,
        Node::Text(t) => render_text_line(t, scope, ctx)?,
        Node::Raw(s) => ctx.push(s)?,
        Node::Comment { text, visible } => {
            if *visible {
                ctx.push("<!--")?;
                ctx.push(text)?;
                ctx.push("-->")?;
            }
        }
        Node::Code(c) => {
            let v = expr::eval(&c.value, scope)?;
            scope.set(&c.name, v);
        }
        Node::If(i) => {
            let cv = expr::eval(&i.cond, scope)?;
            let take_then = if i.invert {
                !expr::truthy(&cv)
            } else {
                expr::truthy(&cv)
            };
            if take_then {
                scope.push();
                render_nodes(&i.then_block, scope, ctx)?;
                scope.pop();
            } else {
                let mut handled = false;
                for (c, blk) in &i.elifs {
                    let v = expr::eval(c, scope)?;
                    if expr::truthy(&v) {
                        scope.push();
                        render_nodes(blk, scope, ctx)?;
                        scope.pop();
                        handled = true;
                        break;
                    }
                }
                if !handled {
                    if let Some(blk) = &i.else_block {
                        scope.push();
                        render_nodes(blk, scope, ctx)?;
                        scope.pop();
                    }
                }
            }
        }
        Node::Each(e) => {
            let iter_val = expr::eval(&e.iter, scope)?;
            match iter_val {
                Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        ctx.bump_iter()?;
                        scope.push();
                        scope.set(&e.var, item.clone());
                        if let Some(idx) = &e.idx {
                            scope.set(
                                idx,
                                serde_json::Number::from_f64(i as f64)
                                    .map(Value::Number)
                                    .unwrap_or(Value::Null),
                            );
                        }
                        render_nodes(&e.block, scope, ctx)?;
                        scope.pop();
                    }
                }
                Value::Object(obj) => {
                    for (k, v) in obj.iter() {
                        ctx.bump_iter()?;
                        scope.push();
                        scope.set(&e.var, v.clone());
                        if let Some(idx) = &e.idx {
                            scope.set(idx, Value::String(k.clone()));
                        }
                        render_nodes(&e.block, scope, ctx)?;
                        scope.pop();
                    }
                }
                Value::Null => {}
                _ => {
                    return Err(PugError::Eval(format!(
                        "cannot iterate over `{iter_val}` at line {}",
                        e.line
                    )))
                }
            }
        }
    }
    Ok(())
}

fn render_tag(t: &Tag, scope: &mut Scope, ctx: &mut RenderCtx) -> PugResult<()> {
    ctx.push("<")?;
    ctx.push(&t.name)?;

    // Merge class= attr with shorthand classes.
    let mut class_list: Vec<String> = t.classes.clone();
    let mut id_val: Option<String> = t.id.clone();
    let mut other_attrs: Vec<(String, Value)> = Vec::new();

    for a in &t.attrs {
        let v = match &a.value {
            AttrValue::True => Value::Bool(true),
            AttrValue::Expr(e) => expr::eval(e, scope)?,
        };
        match a.name.as_str() {
            "class" => append_class(&mut class_list, &v),
            "id" => {
                if let Some(s) = value_truthy_to_str(&v) {
                    id_val = Some(s);
                }
            }
            _ => other_attrs.push((a.name.clone(), v)),
        }
    }

    if let Some(id) = &id_val {
        ctx.push(" id=\"")?;
        ctx.push(&attr_escape(id))?;
        ctx.push("\"")?;
    }
    if !class_list.is_empty() {
        let joined: Vec<String> = class_list
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| attr_escape(s))
            .collect();
        if !joined.is_empty() {
            ctx.push(" class=\"")?;
            ctx.push(&joined.join(" "))?;
            ctx.push("\"")?;
        }
    }
    for (name, v) in &other_attrs {
        emit_attr(name, v, ctx)?;
    }

    let void = is_void(&t.name);
    if t.self_closing || (void && t.children.is_empty() && t.text.is_none()) {
        ctx.push(">")?;
        return Ok(());
    }
    ctx.push(">")?;

    if let Some(text) = &t.text {
        render_text_line(text, scope, ctx)?;
    }
    if t.block_text {
        // children are Text nodes, each on its own line
        for (i, child) in t.children.iter().enumerate() {
            if i > 0 { ctx.push("\n")?; }
            render_node(child, scope, ctx)?;
        }
    } else if !t.children.is_empty() {
        render_nodes(&t.children, scope, ctx)?;
    }

    ctx.push("</")?;
    ctx.push(&t.name)?;
    ctx.push(">")?;
    Ok(())
}

fn append_class(into: &mut Vec<String>, v: &Value) {
    match v {
        Value::String(s) => {
            for piece in s.split_ascii_whitespace() {
                into.push(piece.to_string());
            }
        }
        Value::Array(arr) => {
            for item in arr {
                append_class(into, item);
            }
        }
        Value::Object(obj) => {
            for (k, v) in obj {
                if expr::truthy(v) {
                    into.push(k.clone());
                }
            }
        }
        Value::Bool(true) | Value::Number(_) => {
            into.push(expr::value_to_string(v));
        }
        _ => {}
    }
}

fn emit_attr(name: &str, v: &Value, ctx: &mut RenderCtx) -> PugResult<()> {
    // boolean attrs: true → just `name`, false/null → omit
    if matches!(v, Value::Bool(false) | Value::Null) {
        return Ok(());
    }
    if matches!(v, Value::Bool(true)) {
        ctx.push(" ")?;
        ctx.push(name)?;
        return Ok(());
    }
    ctx.push(" ")?;
    ctx.push(name)?;
    ctx.push("=\"")?;
    ctx.push(&attr_escape(&expr::value_to_string(v)))?;
    ctx.push("\"")?;
    Ok(())
}

fn value_truthy_to_str(v: &Value) -> Option<String> {
    if matches!(v, Value::Null | Value::Bool(false)) {
        None
    } else {
        Some(expr::value_to_string(v))
    }
}

fn render_text_line(t: &TextLine, scope: &mut Scope, ctx: &mut RenderCtx) -> PugResult<()> {
    for seg in &t.segments {
        match seg {
            TextSeg::Literal(s) => ctx.push(&text_escape(s))?,
            TextSeg::Interp(e) => {
                let v = expr::eval(e, scope)?;
                ctx.push(&text_escape(&expr::value_to_string(&v)))?;
            }
            TextSeg::Raw(e) => {
                let v = expr::eval(e, scope)?;
                ctx.push(&expr::value_to_string(&v))?;
            }
        }
    }
    Ok(())
}

// ─── escaping helpers ───────────────────────────────────────────────

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

fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "keygen"
            | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

fn doctype_str(d: &str) -> String {
    match d {
        "" | "html" => "<!DOCTYPE html>".into(),
        "xml" => r#"<?xml version="1.0" encoding="utf-8" ?>"#.into(),
        "transitional" => "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">".into(),
        "strict" => "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">".into(),
        "frameset" => "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Frameset//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-frameset.dtd\">".into(),
        "1.1" => "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.1//EN\" \"http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd\">".into(),
        "basic" => "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML Basic 1.1//EN\" \"http://www.w3.org/TR/xhtml-basic/xhtml-basic11.dtd\">".into(),
        "mobile" => "<!DOCTYPE html PUBLIC \"-//WAPFORUM//DTD XHTML Mobile 1.2//EN\" \"http://www.openmobilealliance.org/tech/DTD/xhtml-mobile12.dtd\">".into(),
        other => format!("<!DOCTYPE {other}>"),
    }
}
