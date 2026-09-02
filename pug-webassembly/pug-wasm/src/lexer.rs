//! Indentation-aware line splitter.
//!
//! The lexer's only job is to break the source into logical lines with
//! their indent level (in columns; a tab counts as one indent unit, but
//! mixed-indent within a file is an error). The parser handles the rest.

use crate::error::{PugError, PugResult};

#[derive(Debug, Clone)]
pub struct Line {
    /// Indent depth in units. A unit is one tab OR a run of N spaces, where
    /// N is determined by the first indented line.
    pub indent: usize,
    /// 1-based line number, for error reporting.
    pub line_no: usize,
    /// The line content with the leading whitespace stripped.
    pub text: String,
}

pub fn lex(source: &str) -> PugResult<Vec<Line>> {
    let mut out = Vec::new();
    let mut space_unit: Option<usize> = None; // spaces per indent
    let mut uses_tabs = false;

    for (i, raw) in source.split('\n').enumerate() {
        let line_no = i + 1;
        // strip trailing CR
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        // pure blank line — skip
        if raw.trim().is_empty() {
            continue;
        }

        // count leading whitespace
        let mut col = 0usize;
        let mut tabs = 0usize;
        let mut spaces = 0usize;
        for ch in raw.chars() {
            match ch {
                '\t' => { tabs += 1; col += 1; }
                ' ' => { spaces += 1; col += 1; }
                _ => break,
            }
        }
        if tabs > 0 && spaces > 0 {
            return Err(PugError::Lex {
                line: line_no,
                col,
                msg: "mixed tabs and spaces in indentation".into(),
            });
        }

        let indent = if tabs > 0 {
            uses_tabs = true;
            if space_unit.is_some() {
                return Err(PugError::Lex {
                    line: line_no,
                    col,
                    msg: "tab indent in file that uses spaces".into(),
                });
            }
            tabs
        } else if spaces > 0 {
            if uses_tabs {
                return Err(PugError::Lex {
                    line: line_no,
                    col,
                    msg: "space indent in file that uses tabs".into(),
                });
            }
            // Lock in the space-per-unit on the first indented line.
            let unit = space_unit.get_or_insert(spaces);
            if spaces % *unit != 0 {
                return Err(PugError::Lex {
                    line: line_no,
                    col,
                    msg: format!(
                        "indent of {spaces} spaces is not a multiple of the file's unit of {unit}"
                    ),
                });
            }
            spaces / *unit
        } else {
            0
        };

        let text = raw[(tabs + spaces)..].to_string();
        out.push(Line { indent, line_no, text });
    }

    Ok(out)
}
