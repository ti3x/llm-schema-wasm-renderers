use crate::expr::Expr;

#[derive(Debug, Clone)]
pub struct Doc {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Doctype(String),
    Tag(Tag),
    Text(TextLine),
    Code(CodeLine),
    If(IfNode),
    Each(EachNode),
    Comment { text: String, visible: bool },
    Raw(String),
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub classes: Vec<String>,
    pub id: Option<String>,
    pub attrs: Vec<Attr>,
    pub self_closing: bool,
    pub block_text: bool,
    pub text: Option<TextLine>,
    pub children: Vec<Node>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub value: AttrValue,
}

#[derive(Debug, Clone)]
pub enum AttrValue {
    /// Bare boolean attribute (e.g. `disabled` with no value)
    True,
    /// Expression to be evaluated at render time
    Expr(Expr),
}

/// A line of text, possibly with `#{...}` / `!{...}` interpolations.
#[derive(Debug, Clone, Default)]
pub struct TextLine {
    pub segments: Vec<TextSeg>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum TextSeg {
    Literal(String),
    /// `#{expr}` — escape on output
    Interp(Expr),
    /// `!{expr}` — raw, not escaped
    Raw(Expr),
}

#[derive(Debug, Clone)]
pub struct CodeLine {
    /// One simple declaration: `var ident = expr` (or `let`/`const`)
    pub name: String,
    pub value: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct IfNode {
    pub cond: Expr,
    /// True if this is `unless` (invert)
    pub invert: bool,
    pub then_block: Vec<Node>,
    /// (cond, block) — `else if` chain
    pub elifs: Vec<(Expr, Vec<Node>)>,
    pub else_block: Option<Vec<Node>>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct EachNode {
    pub var: String,
    pub idx: Option<String>,
    pub iter: Expr,
    pub block: Vec<Node>,
    pub line: usize,
}
