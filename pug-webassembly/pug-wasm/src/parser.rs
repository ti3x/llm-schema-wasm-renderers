//! Parses pug source lines into an AST.

use crate::ast::*;
use crate::error::{PugError, PugResult};
use crate::expr::{self, Expr};
use crate::lexer::Line;

pub fn parse(lines: &[Line]) -> PugResult<Doc> {
    let mut p = State { lines, pos: 0 };
    let nodes = p.parse_block(0)?;
    Ok(Doc { nodes })
}

struct State<'a> {
    lines: &'a [Line],
    pos: usize,
}

impl<'a> State<'a> {
    fn peek(&self) -> Option<&'a Line> { self.lines.get(self.pos) }

    /// Parse all sibling nodes at exactly `indent`. Stops when the next
    /// line is at a smaller indent, or the input is exhausted.
    fn parse_block(&mut self, indent: usize) -> PugResult<Vec<Node>> {
        let mut nodes = Vec::new();
        while let Some(line) = self.peek() {
            if line.indent < indent { break; }
            if line.indent > indent {
                return Err(PugError::Parse {
                    line: line.line_no,
                    msg: format!("unexpected extra indent (expected {indent}, got {})", line.indent),
                });
            }
            self.parse_one(&mut nodes)?;
        }
        Ok(nodes)
    }

    fn parse_one(&mut self, out: &mut Vec<Node>) -> PugResult<()> {
        let line = self.peek().expect("parse_one called with no line");
        let line_no = line.line_no;
        let indent = line.indent;
        let text = line.text.clone();
        let trimmed = text.trim_start();

        // doctype
        if let Some(rest) = trimmed.strip_prefix("doctype") {
            self.pos += 1;
            out.push(Node::Doctype(rest.trim().to_string()));
            return Ok(());
        }
        // silent comment — drops the rest of the indented block
        if trimmed.starts_with("//-") {
            self.pos += 1;
            self.skip_indented_block(indent);
            return Ok(());
        }
        if let Some(rest) = trimmed.strip_prefix("//") {
            self.pos += 1;
            // visible comments may have an indented block of text
            let mut body = rest.trim_start().to_string();
            let block_lines = self.collect_indented_text(indent);
            for bl in block_lines {
                body.push('\n');
                body.push_str(&bl);
            }
            out.push(Node::Comment { text: body, visible: true });
            return Ok(());
        }
        // pipe text
        if let Some(rest) = trimmed.strip_prefix('|') {
            self.pos += 1;
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            let segs = parse_text_segs(rest, line_no)?;
            out.push(Node::Text(TextLine { segments: segs, line: line_no }));
            return Ok(());
        }
        // code line: `- name = expr`  (only the simple var form is supported)
        if let Some(rest) = trimmed.strip_prefix('-') {
            self.pos += 1;
            let code = rest.trim();
            let (name, value_src) = parse_var_decl(code, line_no)?;
            let value = expr::parse(&value_src, line_no)?;
            out.push(Node::Code(CodeLine { name, value, line: line_no }));
            return Ok(());
        }
        // conditionals — recognise the leading word
        if let Some(rest) = strip_word(trimmed, "if") {
            self.pos += 1;
            let cond = expr::parse(rest.trim(), line_no)?;
            let if_node = self.finish_if(cond, false, indent, line_no)?;
            out.push(Node::If(if_node));
            return Ok(());
        }
        if let Some(rest) = strip_word(trimmed, "unless") {
            self.pos += 1;
            let cond = expr::parse(rest.trim(), line_no)?;
            let if_node = self.finish_if(cond, true, indent, line_no)?;
            out.push(Node::If(if_node));
            return Ok(());
        }
        if strip_word(trimmed, "else").is_some() || strip_word(trimmed, "else if").is_some() {
            return Err(PugError::Parse {
                line: line_no,
                msg: "stray `else` / `else if` without matching `if`".into(),
            });
        }
        if let Some(rest) = strip_word(trimmed, "each") {
            self.pos += 1;
            let each = self.parse_each(rest, indent, line_no)?;
            out.push(Node::Each(each));
            return Ok(());
        }
        if let Some(rest) = strip_word(trimmed, "for") {
            self.pos += 1;
            let each = self.parse_each(rest, indent, line_no)?;
            out.push(Node::Each(each));
            return Ok(());
        }

        // tag (default)
        let tag = self.parse_tag_line(trimmed, indent, line_no)?;
        out.push(Node::Tag(tag));
        Ok(())
    }

    fn finish_if(
        &mut self,
        cond: Expr,
        invert: bool,
        indent: usize,
        line: usize,
    ) -> PugResult<IfNode> {
        let then_block = self.parse_block(indent + 1)?;
        let mut elifs = Vec::new();
        let mut else_block: Option<Vec<Node>> = None;
        loop {
            let Some(next) = self.peek() else { break };
            if next.indent != indent { break; }
            let nt = next.text.trim_start();
            if let Some(rest) = strip_word(nt, "else if") {
                let ln = next.line_no;
                self.pos += 1;
                let c = expr::parse(rest.trim(), ln)?;
                let block = self.parse_block(indent + 1)?;
                elifs.push((c, block));
                continue;
            }
            if strip_word(nt, "else").is_some() {
                self.pos += 1;
                let block = self.parse_block(indent + 1)?;
                else_block = Some(block);
                break;
            }
            break;
        }
        Ok(IfNode {
            cond,
            invert,
            then_block,
            elifs,
            else_block,
            line,
        })
    }

    fn parse_each(&mut self, rest: &str, indent: usize, line: usize) -> PugResult<EachNode> {
        // each VAR (',' IDX)? in EXPR
        let in_pos = find_keyword(rest, "in").ok_or_else(|| PugError::Parse {
            line,
            msg: "expected `in` in `each` statement".into(),
        })?;
        let head = rest[..in_pos].trim();
        let iter_src = rest[in_pos + 2..].trim();
        let (var, idx) = if let Some((a, b)) = head.split_once(',') {
            (a.trim().to_string(), Some(b.trim().to_string()))
        } else {
            (head.to_string(), None)
        };
        validate_ident(&var, line)?;
        if let Some(ref i) = idx { validate_ident(i, line)?; }
        let iter = expr::parse(iter_src, line)?;
        let block = self.parse_block(indent + 1)?;
        Ok(EachNode { var, idx, iter, block, line })
    }

    fn parse_tag_line(&mut self, src: &str, indent: usize, line: usize) -> PugResult<Tag> {
        self.pos += 1;
        let mut cur = src;
        // tag name (optional — defaults to "div" if it starts with `.` or `#`)
        let mut name = String::new();
        let mut iter = cur.char_indices().peekable();
        while let Some(&(_, c)) = iter.peek() {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                iter.next();
            } else { break; }
        }
        let consumed = iter.peek().map(|(i, _)| *i).unwrap_or(cur.len());
        cur = &cur[consumed..];
        if name.is_empty() { name = "div".to_string(); }

        let mut classes = Vec::new();
        let mut id: Option<String> = None;

        // class/id shorthand
        loop {
            let bytes = cur.as_bytes();
            if bytes.is_empty() { break; }
            let c = bytes[0];
            if c == b'.' {
                let mut k = 1;
                while k < bytes.len() && is_css_ident_byte(bytes[k]) { k += 1; }
                if k == 1 { break; } // a bare trailing `.` is block-text marker
                classes.push(cur[1..k].to_string());
                cur = &cur[k..];
            } else if c == b'#' {
                let mut k = 1;
                while k < bytes.len() && is_css_ident_byte(bytes[k]) { k += 1; }
                if k == 1 {
                    return Err(PugError::Parse { line, msg: "expected id name after `#`".into() });
                }
                id = Some(cur[1..k].to_string());
                cur = &cur[k..];
            } else { break; }
        }

        // attribute list (...)
        let mut attrs = Vec::new();
        if cur.as_bytes().first() == Some(&b'(') {
            let (parsed, rest) = parse_attr_list(cur, line)?;
            attrs = parsed;
            cur = rest;
        }

        // self-closing slash
        let mut self_closing = false;
        if cur.as_bytes().first() == Some(&b'/') {
            self_closing = true;
            cur = &cur[1..];
        }

        // What's left tells us how the body works.
        let mut block_text = false;
        let mut text: Option<TextLine> = None;

        if cur == "." {
            // bare `.` = block text marker
            block_text = true;
            cur = "";
        } else if let Some(rest) = cur.strip_prefix('.') {
            // bare `.` at end-of-tag (followed by no children or whitespace then newline)
            if rest.trim().is_empty() {
                block_text = true;
                cur = "";
            } else {
                // not a block-text marker — treat as parse error rather than guess
                return Err(PugError::Parse {
                    line,
                    msg: "unexpected `.` after tag (use `.` alone for block text)".into(),
                });
            }
        }

        // `=` and `!=` buffered code
        if let Some(rest) = cur.strip_prefix("!=") {
            let e = expr::parse(rest.trim(), line)?;
            text = Some(TextLine {
                segments: vec![TextSeg::Raw(e)],
                line,
            });
            cur = "";
        } else if let Some(rest) = cur.strip_prefix('=') {
            let e = expr::parse(rest.trim(), line)?;
            text = Some(TextLine {
                segments: vec![TextSeg::Interp(e)],
                line,
            });
            cur = "";
        }

        // Trailing inline text (after a single space)
        if !cur.is_empty() {
            let txt = cur.strip_prefix(' ').unwrap_or(cur);
            let segs = parse_text_segs(txt, line)?;
            text = Some(TextLine { segments: segs, line });
        }

        // Body / children
        let children = if block_text {
            // Collect raw text lines (at indent+1, allowing arbitrary deeper
            // indent treated as relative whitespace) and emit as Text/Raw nodes.
            self.collect_block_text(indent)?
        } else {
            self.parse_block(indent + 1)?
        };

        Ok(Tag {
            name,
            classes,
            id,
            attrs,
            self_closing,
            block_text,
            text,
            children,
            line,
        })
    }

    /// Consume any subsequent lines at deeper indent (their text contents
    /// become children with interpolation).
    fn collect_block_text(&mut self, parent_indent: usize) -> PugResult<Vec<Node>> {
        let mut out = Vec::new();
        let child_indent = parent_indent + 1;
        while let Some(line) = self.peek() {
            if line.indent < child_indent { break; }
            let segs = parse_text_segs(&line.text, line.line_no)?;
            out.push(Node::Text(TextLine { segments: segs, line: line.line_no }));
            self.pos += 1;
        }
        Ok(out)
    }

    /// Discard subsequent lines at deeper indent (used for `//-` silent comments).
    fn skip_indented_block(&mut self, parent_indent: usize) {
        let child_indent = parent_indent + 1;
        while let Some(line) = self.peek() {
            if line.indent < child_indent { break; }
            self.pos += 1;
        }
    }

    /// For `//` (visible comments) — the indented block becomes raw text appended.
    fn collect_indented_text(&mut self, parent_indent: usize) -> Vec<String> {
        let mut out = Vec::new();
        let child_indent = parent_indent + 1;
        while let Some(line) = self.peek() {
            if line.indent < child_indent { break; }
            out.push(line.text.clone());
            self.pos += 1;
        }
        out
    }
}

fn is_css_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Match a leading whole word (followed by EOL or whitespace).
fn strip_word<'a>(src: &'a str, word: &str) -> Option<&'a str> {
    if let Some(rest) = src.strip_prefix(word) {
        if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace()) {
            return Some(rest);
        }
    }
    None
}

/// Locate a whole-word `in` keyword (not inside a sub-expression).
fn find_keyword(src: &str, kw: &str) -> Option<usize> {
    let mut depth = 0i32;
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + kw.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0
            && &bytes[i..i + kw.len()] == kw.as_bytes()
            && (i == 0 || !is_id_byte(bytes[i - 1]))
            && (i + kw.len() == bytes.len() || !is_id_byte(bytes[i + kw.len()]))
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn is_id_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' || b == b'$' }

fn validate_ident(name: &str, line: usize) -> PugResult<()> {
    if name.is_empty() {
        return Err(PugError::Parse { line, msg: "empty identifier".into() });
    }
    let bs = name.as_bytes();
    if !(bs[0].is_ascii_alphabetic() || bs[0] == b'_' || bs[0] == b'$') {
        return Err(PugError::Parse {
            line,
            msg: format!("invalid identifier `{name}`"),
        });
    }
    if !bs.iter().all(|&b| is_id_byte(b)) {
        return Err(PugError::Parse {
            line,
            msg: format!("invalid identifier `{name}`"),
        });
    }
    Ok(())
}

fn parse_var_decl(code: &str, line: usize) -> PugResult<(String, String)> {
    let trimmed = code.trim();
    let after_kw = strip_kw(trimmed, "var")
        .or_else(|| strip_kw(trimmed, "let"))
        .or_else(|| strip_kw(trimmed, "const"))
        .ok_or_else(|| PugError::Parse {
            line,
            msg: "code line must be `- var NAME = EXPR` (or `let` / `const`)".into(),
        })?;
    let rest = after_kw.trim_start();
    let bytes = rest.as_bytes();
    let mut eq_pos = None;
    for i in 0..bytes.len() {
        if bytes[i] == b'=' && bytes.get(i + 1) != Some(&b'=') && (i == 0 || bytes[i - 1] != b'!') {
            eq_pos = Some(i);
            break;
        }
    }
    let eq_pos = eq_pos.ok_or_else(|| PugError::Parse {
        line,
        msg: "expected `=` in code declaration".into(),
    })?;
    let name = rest[..eq_pos].trim();
    validate_ident(name, line)?;
    let expr_src = rest[eq_pos + 1..].trim().to_string();
    Ok((name.to_string(), expr_src))
}

fn strip_kw<'a>(src: &'a str, kw: &str) -> Option<&'a str> {
    let rest = src.strip_prefix(kw)?;
    if rest.starts_with(|c: char| c.is_whitespace()) {
        Some(rest)
    } else {
        None
    }
}

// ─── attribute-list parser  ──────────────────────────────────────────

fn parse_attr_list<'s>(src: &'s str, line: usize) -> PugResult<(Vec<Attr>, &'s str)> {
    let bytes = src.as_bytes();
    assert_eq!(bytes[0], b'(');
    let mut i = 1usize;
    let mut attrs = Vec::new();
    loop {
        // skip whitespace + commas
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(PugError::Parse { line, msg: "unterminated attribute list".into() });
        }
        if bytes[i] == b')' { i += 1; break; }

        // attr name
        let name_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_' || bytes[i] == b':')
        { i += 1; }
        if i == name_start {
            return Err(PugError::Parse { line, msg: "expected attribute name".into() });
        }
        let name = src[name_start..i].to_string();

        // skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }

        let value = if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
            let (expr_src, consumed) = read_attr_value(&src[i..])?;
            i += consumed;
            AttrValue::Expr(expr::parse(expr_src.trim(), line)?)
        } else {
            AttrValue::True
        };
        attrs.push(Attr { name, value });
    }
    Ok((attrs, &src[i..]))
}

/// Read a single attribute value expression up to a top-level `,` or `)`.
/// Handles balanced quotes / parens / brackets / braces.
fn read_attr_value(src: &str) -> PugResult<(&str, usize)> {
    let bytes = src.as_bytes();
    let mut depth_paren = 0i32;
    let mut depth_brack = 0i32;
    let mut depth_brace = 0i32;
    let mut in_str: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
            if c == q { in_str = None; }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' => in_str = Some(c),
            b'(' => depth_paren += 1,
            b')' => {
                if depth_paren == 0 { break; }
                depth_paren -= 1;
            }
            b'[' => depth_brack += 1,
            b']' => depth_brack -= 1,
            b'{' => depth_brace += 1,
            b'}' => depth_brace -= 1,
            b',' if depth_paren == 0 && depth_brack == 0 && depth_brace == 0 => break,
            _ => {}
        }
        i += 1;
    }
    Ok((&src[..i], i))
}

// ─── text-line interpolation parser  ─────────────────────────────────

pub fn parse_text_segs(src: &str, line: usize) -> PugResult<Vec<TextSeg>> {
    let mut segs = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut buf = String::new();
    while i < bytes.len() {
        let c = bytes[i];
        // `#{expr}` escaped interpolation
        if c == b'#' && bytes.get(i + 1) == Some(&b'{') {
            if !buf.is_empty() {
                segs.push(TextSeg::Literal(std::mem::take(&mut buf)));
            }
            let (expr_src, consumed) = read_brace_balanced(&src[i + 2..])?;
            segs.push(TextSeg::Interp(expr::parse(expr_src, line)?));
            i += 2 + consumed + 1;
            continue;
        }
        // `!{expr}` raw interpolation
        if c == b'!' && bytes.get(i + 1) == Some(&b'{') {
            if !buf.is_empty() {
                segs.push(TextSeg::Literal(std::mem::take(&mut buf)));
            }
            let (expr_src, consumed) = read_brace_balanced(&src[i + 2..])?;
            segs.push(TextSeg::Raw(expr::parse(expr_src, line)?));
            i += 2 + consumed + 1;
            continue;
        }
        // escape: `\#{` keeps the literal
        if c == b'\\' && bytes.get(i + 1).map_or(false, |&b| b == b'#' || b == b'!') {
            buf.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        buf.push(c as char);
        i += 1;
    }
    if !buf.is_empty() {
        segs.push(TextSeg::Literal(buf));
    }
    Ok(segs)
}

fn read_brace_balanced(src: &str) -> PugResult<(&str, usize)> {
    let bytes = src.as_bytes();
    let mut depth = 1i32;
    let mut in_str: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
            if c == q { in_str = None; }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' => in_str = Some(c),
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 { return Ok((&src[..i], i)); }
            }
            _ => {}
        }
        i += 1;
    }
    Err(PugError::Parse { line: 0, msg: "unterminated `#{...}` / `!{...}`".into() })
}
