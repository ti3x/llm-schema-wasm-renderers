//! Restricted expression parser + evaluator.
//!
//! Security model: the grammar deliberately excludes any way to express
//! arbitrary code execution. There is no `function` / `=>` / `new` /
//! `typeof` / assignment. Method calls are only legal against a tiny
//! whitelist on known value kinds. Identifiers that resolve to anything
//! callable beyond that are not addressable.

use crate::error::{PugError, PugResult};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Expr {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Ident(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
}

// ─── Tokenizer (just for expressions) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    // operators / punct
    LParen, RParen, LBrack, RBrack, LBrace, RBrace,
    Comma, Colon, Dot, Question,
    Plus, Minus, Star, Slash, Percent, Bang,
    EqEq, NotEq, StrictEq, StrictNotEq,
    Lt, Gt, Lte, Gte,
    AndAnd, OrOr,
    Eof,
}

fn tokenize(src: &str, line: usize) -> PugResult<Vec<Tok>> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        // whitespace
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // banned tokens — fail fast
        if c == b'=' && bytes.get(i + 1) == Some(&b'>') {
            return Err(PugError::Expr {
                line,
                msg: "arrow function `=>` is not allowed".into(),
            });
        }
        if c == b'=' && bytes.get(i + 1) != Some(&b'=') {
            return Err(PugError::Expr {
                line,
                msg: "assignment `=` is not allowed".into(),
            });
        }
        // numbers
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let n: f64 = src[start..i]
                .parse()
                .map_err(|_| PugError::Expr { line, msg: "bad number".into() })?;
            out.push(Tok::Num(n));
            continue;
        }
        // strings
        if c == b'"' || c == b'\'' {
            let quote = c;
            i += 1;
            let start = i;
            let mut s = String::new();
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    let esc = bytes[i + 1];
                    s.push(match esc {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'\\' => '\\',
                        b'\'' => '\'',
                        b'"' => '"',
                        _ => esc as char,
                    });
                    i += 2;
                    continue;
                }
                s.push(bytes[i] as char);
                i += 1;
            }
            if i >= bytes.len() {
                return Err(PugError::Expr {
                    line,
                    msg: format!("unterminated string starting at offset {start}"),
                });
            }
            i += 1; // closing quote
            out.push(Tok::Str(s));
            continue;
        }
        // identifiers / keywords
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }
            let ident = &src[start..i];
            // banned identifiers (parse-time hard reject)
            match ident {
                "function" | "new" | "delete" | "typeof" | "instanceof" | "void"
                | "yield" | "await" | "throw" | "this" | "arguments" | "eval"
                | "constructor" | "__proto__" | "prototype" => {
                    return Err(PugError::Expr {
                        line,
                        msg: format!("identifier `{ident}` is not allowed"),
                    });
                }
                _ => {}
            }
            out.push(Tok::Ident(ident.to_string()));
            continue;
        }

        // multi-char operators
        let two = if i + 1 < bytes.len() {
            Some(&bytes[i..i + 2])
        } else {
            None
        };
        let three = if i + 2 < bytes.len() {
            Some(&bytes[i..i + 3])
        } else {
            None
        };
        if let Some(t) = three {
            match t {
                b"===" => {
                    out.push(Tok::StrictEq);
                    i += 3;
                    continue;
                }
                b"!==" => {
                    out.push(Tok::StrictNotEq);
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        if let Some(t) = two {
            match t {
                b"==" => { out.push(Tok::EqEq); i += 2; continue; }
                b"!=" => { out.push(Tok::NotEq); i += 2; continue; }
                b"<=" => { out.push(Tok::Lte); i += 2; continue; }
                b">=" => { out.push(Tok::Gte); i += 2; continue; }
                b"&&" => { out.push(Tok::AndAnd); i += 2; continue; }
                b"||" => { out.push(Tok::OrOr); i += 2; continue; }
                _ => {}
            }
        }

        // single-char
        let tok = match c {
            b'(' => Tok::LParen,
            b')' => Tok::RParen,
            b'[' => Tok::LBrack,
            b']' => Tok::RBrack,
            b'{' => Tok::LBrace,
            b'}' => Tok::RBrace,
            b',' => Tok::Comma,
            b':' => Tok::Colon,
            b'.' => Tok::Dot,
            b'?' => Tok::Question,
            b'+' => Tok::Plus,
            b'-' => Tok::Minus,
            b'*' => Tok::Star,
            b'/' => Tok::Slash,
            b'%' => Tok::Percent,
            b'!' => Tok::Bang,
            b'<' => Tok::Lt,
            b'>' => Tok::Gt,
            other => {
                return Err(PugError::Expr {
                    line,
                    msg: format!("unexpected character `{}` in expression", other as char),
                });
            }
        };
        out.push(tok);
        i += 1;
    }
    out.push(Tok::Eof);
    Ok(out)
}

// ─── Parser ────────────────────────────────────────────────────────────

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    line: usize,
}

pub fn parse(src: &str, line: usize) -> PugResult<Expr> {
    let toks = tokenize(src, line)?;
    let mut p = Parser { toks, pos: 0, line };
    let e = p.parse_expr()?;
    if !matches!(p.peek(), Tok::Eof) {
        return Err(PugError::Expr {
            line,
            msg: format!("trailing tokens after expression: {:?}", p.peek()),
        });
    }
    Ok(e)
}

impl Parser {
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn bump(&mut self) -> Tok { let t = self.toks[self.pos].clone(); self.pos += 1; t }
    fn eat(&mut self, t: &Tok) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(t) {
            self.bump();
            true
        } else { false }
    }
    fn err(&self, msg: impl Into<String>) -> PugError {
        PugError::Expr { line: self.line, msg: msg.into() }
    }

    fn parse_expr(&mut self) -> PugResult<Expr> { self.parse_ternary() }

    fn parse_ternary(&mut self) -> PugResult<Expr> {
        let cond = self.parse_or()?;
        if matches!(self.peek(), Tok::Question) {
            self.bump();
            let a = self.parse_expr()?;
            if !matches!(self.peek(), Tok::Colon) {
                return Err(self.err("expected `:` in ternary"));
            }
            self.bump();
            let b = self.parse_expr()?;
            return Ok(Expr::Ternary(Box::new(cond), Box::new(a), Box::new(b)));
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Tok::OrOr) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary(BinOp::Or, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_eq()?;
        while matches!(self.peek(), Tok::AndAnd) {
            self.bump();
            let rhs = self.parse_eq()?;
            lhs = Expr::Binary(BinOp::And, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_eq(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_cmp()?;
        loop {
            let op = match self.peek() {
                Tok::EqEq | Tok::StrictEq => BinOp::Eq,
                Tok::NotEq | Tok::StrictNotEq => BinOp::Neq,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_cmp()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Tok::Lt => BinOp::Lt,
                Tok::Gt => BinOp::Gt,
                Tok::Lte => BinOp::Lte,
                Tok::Gte => BinOp::Gte,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_add()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_mul()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> PugResult<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PugResult<Expr> {
        match self.peek() {
            Tok::Bang => {
                self.bump();
                let e = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Not, Box::new(e)))
            }
            Tok::Minus => {
                self.bump();
                let e = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(e)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> PugResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.bump();
                    let name = match self.bump() {
                        Tok::Ident(s) => s,
                        other => {
                            return Err(self.err(format!("expected identifier after `.`, got {other:?}")));
                        }
                    };
                    // Method call?
                    if matches!(self.peek(), Tok::LParen) {
                        self.bump();
                        let mut args = Vec::new();
                        if !matches!(self.peek(), Tok::RParen) {
                            args.push(self.parse_expr()?);
                            while self.eat(&Tok::Comma) {
                                args.push(self.parse_expr()?);
                            }
                        }
                        if !self.eat(&Tok::RParen) {
                            return Err(self.err("expected `)` after method call"));
                        }
                        e = Expr::Call {
                            receiver: Box::new(e),
                            method: name,
                            args,
                        };
                    } else {
                        e = Expr::Member(Box::new(e), name);
                    }
                }
                Tok::LBrack => {
                    self.bump();
                    let idx = self.parse_expr()?;
                    if !self.eat(&Tok::RBrack) {
                        return Err(self.err("expected `]`"));
                    }
                    e = Expr::Index(Box::new(e), Box::new(idx));
                }
                Tok::LParen => {
                    // Bare function calls like `foo()` are not allowed —
                    // only method calls on whitelisted receivers.
                    return Err(self.err("function calls are not allowed; only whitelisted methods like `.length` / `.toUpperCase()`"));
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> PugResult<Expr> {
        let tok = self.bump();
        match tok {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Ident(s) => match s.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "null" | "undefined" => Ok(Expr::Null),
                _ => Ok(Expr::Ident(s)),
            },
            Tok::LParen => {
                let e = self.parse_expr()?;
                if !self.eat(&Tok::RParen) {
                    return Err(self.err("expected `)`"));
                }
                Ok(e)
            }
            Tok::LBrack => {
                let mut items = Vec::new();
                if !matches!(self.peek(), Tok::RBrack) {
                    items.push(self.parse_expr()?);
                    while self.eat(&Tok::Comma) {
                        if matches!(self.peek(), Tok::RBrack) { break; }
                        items.push(self.parse_expr()?);
                    }
                }
                if !self.eat(&Tok::RBrack) {
                    return Err(self.err("expected `]`"));
                }
                Ok(Expr::Array(items))
            }
            Tok::LBrace => {
                let mut entries = Vec::new();
                if !matches!(self.peek(), Tok::RBrace) {
                    loop {
                        let key = match self.bump() {
                            Tok::Ident(s) => s,
                            Tok::Str(s) => s,
                            other => return Err(self.err(format!("expected object key, got {other:?}"))),
                        };
                        if !self.eat(&Tok::Colon) {
                            return Err(self.err("expected `:` in object literal"));
                        }
                        let v = self.parse_expr()?;
                        entries.push((key, v));
                        if !self.eat(&Tok::Comma) { break; }
                        if matches!(self.peek(), Tok::RBrace) { break; }
                    }
                }
                if !self.eat(&Tok::RBrace) {
                    return Err(self.err("expected `}`"));
                }
                Ok(Expr::Object(entries))
            }
            other => Err(self.err(format!("unexpected token {other:?} in expression"))),
        }
    }
}

// ─── Evaluator ─────────────────────────────────────────────────────────

/// A scope used during rendering: a stack of frames mapping local names
/// to values. Used for `- var x = ...` bindings, `each` loop variables,
/// and the initial JSON locals.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    frames: Vec<HashMap<String, Value>>,
}

impl Scope {
    pub fn new() -> Self { Self::default() }
    pub fn push(&mut self) { self.frames.push(HashMap::new()); }
    pub fn pop(&mut self) { self.frames.pop(); }
    pub fn set(&mut self, name: &str, val: Value) {
        if let Some(f) = self.frames.last_mut() {
            f.insert(name.to_string(), val);
        } else {
            let mut f = HashMap::new();
            f.insert(name.to_string(), val);
            self.frames.push(f);
        }
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        for f in self.frames.iter().rev() {
            if let Some(v) = f.get(name) {
                return Some(v);
            }
        }
        None
    }
    /// Seed the bottom frame with a JSON object's top-level keys.
    pub fn seed_from(&mut self, locals: &Value) {
        let mut f = HashMap::new();
        if let Some(obj) = locals.as_object() {
            for (k, v) in obj {
                f.insert(k.clone(), v.clone());
            }
        }
        self.frames.push(f);
    }
}

pub fn eval(expr: &Expr, scope: &Scope) -> PugResult<Value> {
    match expr {
        Expr::Null => Ok(Value::Null),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Num(n) => Ok(serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null)),
        Expr::Str(s) => Ok(Value::String(s.clone())),
        Expr::Ident(name) => Ok(scope.get(name).cloned().unwrap_or(Value::Null)),
        Expr::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items { out.push(eval(it, scope)?); }
            Ok(Value::Array(out))
        }
        Expr::Object(entries) => {
            let mut m = Map::new();
            for (k, v) in entries {
                m.insert(k.clone(), eval(v, scope)?);
            }
            Ok(Value::Object(m))
        }
        Expr::Unary(op, inner) => {
            let v = eval(inner, scope)?;
            match op {
                UnOp::Not => Ok(Value::Bool(!truthy(&v))),
                UnOp::Neg => Ok(serde_json::Number::from_f64(-to_number(&v))
                    .map(Value::Number)
                    .unwrap_or(Value::Null)),
            }
        }
        Expr::Binary(op, l, r) => {
            // short-circuit for &&/||
            match op {
                BinOp::And => {
                    let lv = eval(l, scope)?;
                    return if !truthy(&lv) { Ok(lv) } else { eval(r, scope) };
                }
                BinOp::Or => {
                    let lv = eval(l, scope)?;
                    return if truthy(&lv) { Ok(lv) } else { eval(r, scope) };
                }
                _ => {}
            }
            let lv = eval(l, scope)?;
            let rv = eval(r, scope)?;
            Ok(apply_bin(*op, &lv, &rv))
        }
        Expr::Ternary(c, a, b) => {
            if truthy(&eval(c, scope)?) {
                eval(a, scope)
            } else {
                eval(b, scope)
            }
        }
        Expr::Member(obj, name) => {
            // Re-block dangerous keys at eval time (belt-and-braces;
            // they're already rejected at tokenize time).
            if matches!(name.as_str(), "constructor" | "__proto__" | "prototype") {
                return Err(PugError::Eval(format!("access to `{name}` is not allowed")));
            }
            let v = eval(obj, scope)?;
            Ok(member(&v, name))
        }
        Expr::Index(obj, idx) => {
            let v = eval(obj, scope)?;
            let i = eval(idx, scope)?;
            // String keys go through `member` (with the same blocklist).
            if let Some(s) = i.as_str() {
                if matches!(s, "constructor" | "__proto__" | "prototype") {
                    return Err(PugError::Eval(format!("access to `{s}` is not allowed")));
                }
                Ok(member(&v, s))
            } else if let Some(n) = i.as_f64() {
                let idx = n as usize;
                if let Some(arr) = v.as_array() {
                    Ok(arr.get(idx).cloned().unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            } else {
                Ok(Value::Null)
            }
        }
        Expr::Call { receiver, method, args } => {
            let recv = eval(receiver, scope)?;
            let mut evald = Vec::with_capacity(args.len());
            for a in args { evald.push(eval(a, scope)?); }
            call_method(&recv, method, &evald)
        }
    }
}

fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Object(m) => m.get(name).cloned().unwrap_or(Value::Null),
        Value::Array(a) => {
            if name == "length" {
                serde_json::Number::from_f64(a.len() as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else if let Ok(idx) = name.parse::<usize>() {
                a.get(idx).cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Value::String(s) => {
            if name == "length" {
                serde_json::Number::from_f64(s.chars().count() as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

fn call_method(recv: &Value, method: &str, args: &[Value]) -> PugResult<Value> {
    match recv {
        Value::String(s) => match method {
            "toUpperCase" if args.is_empty() => Ok(Value::String(s.to_uppercase())),
            "toLowerCase" if args.is_empty() => Ok(Value::String(s.to_lowercase())),
            "trim" if args.is_empty() => Ok(Value::String(s.trim().to_string())),
            "includes" if args.len() == 1 => {
                let needle = args[0].as_str().unwrap_or("");
                Ok(Value::Bool(s.contains(needle)))
            }
            _ => Err(PugError::Eval(format!(
                "method `.{method}(...)` is not allowed on strings"
            ))),
        },
        Value::Array(a) => match method {
            "includes" if args.len() == 1 => Ok(Value::Bool(a.contains(&args[0]))),
            "join" if args.len() <= 1 => {
                let sep = args.get(0).and_then(|v| v.as_str()).unwrap_or(",");
                let parts: Vec<String> = a.iter().map(value_to_string).collect();
                Ok(Value::String(parts.join(sep)))
            }
            _ => Err(PugError::Eval(format!(
                "method `.{method}(...)` is not allowed on arrays"
            ))),
        },
        Value::Object(m) => match method {
            "hasOwnProperty" if args.len() == 1 => {
                let key = args[0].as_str().unwrap_or("");
                Ok(Value::Bool(m.contains_key(key)))
            }
            _ => Err(PugError::Eval(format!(
                "method `.{method}(...)` is not allowed on objects"
            ))),
        },
        _ => Err(PugError::Eval(format!(
            "method `.{method}(...)` is not allowed on this value"
        ))),
    }
}

fn apply_bin(op: BinOp, l: &Value, r: &Value) -> Value {
    use BinOp::*;
    match op {
        Add => {
            // Mimic JS-ish: if either is a string, concatenate.
            if l.is_string() || r.is_string() {
                Value::String(format!("{}{}", value_to_string(l), value_to_string(r)))
            } else {
                num(to_number(l) + to_number(r))
            }
        }
        Sub => num(to_number(l) - to_number(r)),
        Mul => num(to_number(l) * to_number(r)),
        Div => {
            let d = to_number(r);
            if d == 0.0 { Value::Null } else { num(to_number(l) / d) }
        }
        Mod => {
            let d = to_number(r);
            if d == 0.0 { Value::Null } else { num(to_number(l) % d) }
        }
        Eq => Value::Bool(loose_eq(l, r)),
        Neq => Value::Bool(!loose_eq(l, r)),
        Lt => Value::Bool(to_number(l) < to_number(r)),
        Gt => Value::Bool(to_number(l) > to_number(r)),
        Lte => Value::Bool(to_number(l) <= to_number(r)),
        Gte => Value::Bool(to_number(l) >= to_number(r)),
        And | Or => unreachable!("short-circuited above"),
    }
}

fn num(n: f64) -> Value {
    serde_json::Number::from_f64(n)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(true) => 1.0,
        Value::Bool(false) => 0.0,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            // Match JS: integer-valued floats stringify without `.0`.
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

fn loose_eq(l: &Value, r: &Value) -> bool {
    match (l, r) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        // Fall back to numeric coercion for cross-type compares.
        _ => to_number(l) == to_number(r),
    }
}
