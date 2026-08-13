//! Parsing the tree format.

use std::sync::Arc;

use crate::error::SyntaxError;
use crate::span::Span;
use crate::tree::Tree;

/// A node under construction. The parser appends to nodes that are already
/// deep inside the tree, so it builds this flat arena first and freezes it into
/// immutable [`Tree`] nodes at the end.
struct Raw {
    kind: Arc<str>,
    value: Arc<str>,
    kids: Vec<usize>,
    span: Span,
}

/// Parses the tree format.
///
/// Returns a list node spanning the whole source, holding whatever the source
/// declared at the outermost level.
///
/// ```
/// let tree = tree2::parse("server\n\thost \\0.0.0.0\n\tport \\8080\n", "conf")?;
/// assert_eq!(tree.kids()[0].kind(), "server");
/// assert_eq!(tree.kids()[0].kids().len(), 2);
/// # Ok::<_, tree2::SyntaxError>(())
/// ```
///
/// # Errors
///
/// Fails on a stray space or tab, on indentation that jumps or falls too far,
/// and on a last line without its newline.
pub fn parse(source: impl Into<Arc<str>>, uri: impl Into<Arc<str>>) -> Result<Tree, SyntaxError> {
    parse_shared(source.into(), uri.into())
}

fn parse_shared(source: Arc<str>, uri: Arc<str>) -> Result<Tree, SyntaxError> {
    let bytes = source.as_bytes();
    let whole = Span::entire(uri, source.clone());
    let empty: Arc<str> = Arc::from("");

    let mut arena = vec![Raw {
        kind: empty.clone(),
        value: empty.clone(),
        kids: Vec::new(),
        span: whole.clone(),
    }];
    let mut stack = vec![0usize];

    // Byte offsets drive the scan; code point offsets drive the columns. Every
    // delimiter is ASCII, so a byte at a time is safe as long as the code point
    // counter skips UTF-8 continuation bytes.
    let mut pos = 0usize;
    let mut col = 0usize;
    let mut row = 0usize;
    let mut min_indent = 0usize;

    while pos < bytes.len() {
        let line_start = pos;
        let line_col = col;
        let mut indent = 0usize;
        row += 1;

        while pos < bytes.len() && bytes[pos] == b'\t' {
            indent += 1;
            pos += 1;
            col += 1;
        }

        // Until the first node shows up, every line resets the base indent, so
        // it ends up holding the indentation of the first line that produces a
        // node.
        if arena[0].kids.is_empty() {
            min_indent = indent;
        }
        let indent = indent as isize - min_indent as isize;

        // The depth is checked against the stack the previous line left behind.
        if indent < 0 || indent as usize >= stack.len() {
            let span = whole.span(row, 1, col - line_col);

            while pos < bytes.len() && bytes[pos] != b'\n' {
                advance(bytes, &mut pos, &mut col);
            }

            let line = &source[line_start..pos];

            // Unconditionally, unlike the reference, which lets a dedented last
            // line off when the document does not end in a newline — and then
            // either loses the line or crashes. Appending a newline to a source
            // never changes which error it reports.
            let reason = if indent < 0 {
                "Too few tabs"
            } else {
                "Too many tabs"
            };

            return Err(SyntaxError::new(reason, line, span));
        }

        let indent = indent as usize;
        stack.truncate(indent + 1);
        let mut parent = stack[indent];

        // Struct nodes, each the parent of the next.
        while pos < bytes.len() && bytes[pos] != b'\\' && bytes[pos] != b'\n' {
            let gap_start = pos;
            let gap_col = col;

            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
                col += 1;
            }

            // One space between nodes was consumed at the end of the previous
            // turn, so anything found here is one separator too many.
            if pos > gap_start {
                let line_end = memchr(bytes, b'\n', pos).unwrap_or(bytes.len());
                let span = whole.span(row, gap_col - line_col + 1, col - gap_col);
                return Err(SyntaxError::new(
                    "Wrong nodes separator",
                    &source[line_start..line_end],
                    span,
                ));
            }

            let kind_start = pos;
            let kind_col = col;

            while pos < bytes.len() && !matches!(bytes[pos], b'\\' | b' ' | b'\t' | b'\n') {
                advance(bytes, &mut pos, &mut col);
            }

            if pos > kind_start {
                let span = whole.span(row, kind_col - line_col + 1, col - kind_col);
                parent = attach(
                    &mut arena,
                    parent,
                    Raw {
                        kind: Arc::from(&source[kind_start..pos]),
                        value: empty.clone(),
                        kids: Vec::new(),
                        span,
                    },
                );
            }

            if pos < bytes.len() && bytes[pos] == b' ' {
                pos += 1;
                col += 1;
            }
        }

        // A data node runs to the end of the line, backslash excluded.
        if pos < bytes.len() && bytes[pos] == b'\\' {
            let data_start = pos;
            let data_col = col;

            while pos < bytes.len() && bytes[pos] != b'\n' {
                advance(bytes, &mut pos, &mut col);
            }

            let span = whole.span(row, data_col - line_col + 2, col - data_col - 1);
            parent = attach(
                &mut arena,
                parent,
                Raw {
                    kind: empty.clone(),
                    value: Arc::from(&source[data_start + 1..pos]),
                    kids: Vec::new(),
                    span,
                },
            );
        }

        if pos == bytes.len() {
            let span = whole.span(row, col - line_col + 1, 1);
            return Err(SyntaxError::new(
                "Unexpected EOF, LF required",
                &source[line_start..],
                span,
            ));
        }

        stack.push(parent);
        pos += 1;
        col += 1;
    }

    Ok(freeze(arena, whole))
}

/// Adds a node to the arena as a child of `parent`, and returns it as the new
/// parent.
fn attach(arena: &mut Vec<Raw>, parent: usize, node: Raw) -> usize {
    let id = arena.len();
    arena.push(node);
    arena[parent].kids.push(id);
    id
}

/// Turns the arena into immutable nodes, children first. Every node's children
/// were pushed after it, so one backwards pass is enough — and it keeps the
/// depth of the tree off the call stack.
fn freeze(arena: Vec<Raw>, whole: Span) -> Tree {
    let filler = Tree::list([], whole);
    let mut built = vec![filler.clone(); arena.len()];

    for (id, raw) in arena.into_iter().enumerate().rev() {
        let kids: Box<[Tree]> = raw
            .kids
            .iter()
            .map(|&kid| std::mem::replace(&mut built[kid], filler.clone()))
            .collect();

        built[id] = Tree::raw(raw.kind, raw.value, kids, raw.span);
    }

    built.into_iter().next().unwrap_or(filler)
}

/// Steps over one byte, counting code points rather than bytes.
fn advance(bytes: &[u8], pos: &mut usize, col: &mut usize) {
    if bytes[*pos] & 0xC0 != 0x80 {
        *col += 1;
    }
    *pos += 1;
}

fn memchr(haystack: &[u8], needle: u8, from: usize) -> Option<usize> {
    haystack[from..]
        .iter()
        .position(|&byte| byte == needle)
        .map(|found| found + from)
}
