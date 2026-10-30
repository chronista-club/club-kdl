//! KDL v2 text writer for the [`crate::ast`] tree.
//!
//! Output is normalized: one node per line, four-space indentation, entries
//! in tree order, strings written as bare identifiers whenever that is valid
//! and quoted (with escapes) otherwise.

use std::fmt::Write as _;

use crate::ast::{Document, Entry, Node, Value};

const INDENT: &str = "    ";

pub(crate) fn write_document(out: &mut String, doc: &Document<'_>) {
    for node in doc.nodes() {
        write_node(out, node, 0);
    }
}

pub(crate) fn write_node(out: &mut String, node: &Node<'_>, depth: usize) {
    write_node_with_indent::<true>(out, node, depth);
}

#[cfg(feature = "compact")]
pub(crate) fn write_document_unindented(out: &mut String, doc: &Document<'_>) {
    for node in doc.nodes() {
        write_node_unindented(out, node);
    }
}

#[cfg(feature = "compact")]
pub(crate) fn write_node_unindented(out: &mut String, node: &Node<'_>) {
    write_node_with_indent::<false>(out, node, 0);
}

fn write_node_with_indent<const INDENTED: bool>(out: &mut String, node: &Node<'_>, depth: usize) {
    if INDENTED {
        for _ in 0..depth {
            out.push_str(INDENT);
        }
    }
    if let Some(ty) = node.ty() {
        write_type(out, ty);
    }
    write_string(out, node.name());
    for entry in node.entries() {
        out.push(' ');
        write_entry(out, entry);
    }
    if !node.children().is_empty() {
        out.push_str(" {\n");
        for child in node.children() {
            write_node_with_indent::<INDENTED>(out, child, depth + 1);
        }
        if INDENTED {
            for _ in 0..depth {
                out.push_str(INDENT);
            }
        }
        out.push('}');
    }
    out.push('\n');
}

/// Streams one node straight to text, without building a [`Node`].
///
/// Derived [`KdlSerialize`](crate::KdlSerialize) impls drive this: the node
/// name is written on creation, entries are appended in order, and the
/// children block (` {` … `}`) is opened lazily by the first child, so a node
/// without children stays on one line — the same text the tree path writes.
pub struct NodeWriter<'o> {
    out: &'o mut String,
    depth: usize,
    has_children: bool,
    /// A document root writes neither a name, entries nor braces; its
    /// children are top-level nodes.
    root: bool,
}

impl<'o> NodeWriter<'o> {
    /// Start a node named `name` at `depth`.
    #[inline]
    pub fn new(out: &'o mut String, depth: usize, name: &str) -> Self {
        for _ in 0..depth {
            out.push_str(INDENT);
        }
        write_string(out, name);
        Self {
            out,
            depth,
            has_children: false,
            root: false,
        }
    }

    /// A document root: children are written as top-level nodes.
    #[inline]
    pub fn document(out: &'o mut String) -> Self {
        Self {
            out,
            depth: 0,
            has_children: false,
            root: true,
        }
    }

    /// Append a positional argument.
    #[inline]
    pub fn arg<V: crate::ToKdlValue + ?Sized>(&mut self, value: &V) {
        if !self.root {
            self.out.push(' ');
            write_value(self.out, &value.to_kdl_value());
        }
    }

    /// Append a `key=value` property.
    #[inline]
    pub fn prop<V: crate::ToKdlValue + ?Sized>(&mut self, key: &str, value: &V) {
        if !self.root {
            self.out.push(' ');
            write_string(self.out, key);
            self.out.push('=');
            write_value(self.out, &value.to_kdl_value());
        }
    }

    /// Append a ready-made entry.
    #[inline]
    pub fn entry(&mut self, entry: &Entry<'_>) {
        if !self.root {
            self.out.push(' ');
            write_entry(self.out, entry);
        }
    }

    /// Open the children block if needed and return the buffer and depth a
    /// child node must be written at.
    #[inline]
    pub fn nested(&mut self) -> (&mut String, usize) {
        if self.root {
            return (self.out, 0);
        }
        if !self.has_children {
            self.out.push_str(" {\n");
            self.has_children = true;
        }
        (self.out, self.depth + 1)
    }

    /// Start a child node named `name`.
    #[inline]
    pub fn child(&mut self, name: &str) -> NodeWriter<'_> {
        let (out, depth) = self.nested();
        NodeWriter::new(out, depth, name)
    }

    /// Write a ready-made child node.
    #[inline]
    pub fn child_node(&mut self, node: &Node<'_>) {
        let (out, depth) = self.nested();
        write_node(out, node, depth);
    }

    /// Close the node: the children block if one was opened, then the newline.
    #[inline]
    pub fn finish(self) {
        if self.root {
            return;
        }
        if self.has_children {
            for _ in 0..self.depth {
                self.out.push_str(INDENT);
            }
            self.out.push('}');
        }
        self.out.push('\n');
    }
}

fn write_entry(out: &mut String, entry: &Entry<'_>) {
    if let Some(name) = entry.name() {
        write_string(out, name);
        out.push('=');
    }
    if let Some(ty) = entry.ty() {
        write_type(out, ty);
    }
    write_value(out, entry.value());
}

fn write_type(out: &mut String, ty: &str) {
    out.push('(');
    write_string(out, ty);
    out.push(')');
}

pub(crate) fn write_value(out: &mut String, value: &Value<'_>) {
    match value {
        Value::String(s) => write_string(out, s),
        Value::Integer(i) => write_int(out, *i),
        Value::Float(f) => write_float(out, *f),
        Value::Bool(true) => out.push_str("#true"),
        Value::Bool(false) => out.push_str("#false"),
        Value::Null => out.push_str("#null"),
    }
}

fn write_int(out: &mut String, i: i128) {
    let negative = i < 0;
    let mut n = i.unsigned_abs();
    let mut buf = [0u8; 40];
    let mut pos = buf.len();
    loop {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if negative {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&buf[pos..]).unwrap());
}

fn write_float(out: &mut String, f: f64) {
    if f.is_nan() {
        out.push_str("#nan");
    } else if f == f64::INFINITY {
        out.push_str("#inf");
    } else if f == f64::NEG_INFINITY {
        out.push_str("#-inf");
    } else {
        // `Debug` always keeps a fractional part or exponent (`1.0`, `1e21`,
        // `1e-7`), so the value reads back as a float.
        let _ = write!(out, "{f:?}");
    }
}

/// Write `s` as a bare identifier if possible, otherwise as a quoted string.
pub(crate) fn write_string(out: &mut String, s: &str) {
    if is_bare_identifier(s) {
        out.push_str(s);
    } else {
        write_quoted(out, s);
    }
}

fn is_bare_identifier(s: &str) -> bool {
    let b = s.as_bytes();
    let Some(&first) = b.first() else {
        return false;
    };
    // Must not read back as a number: digit, sign+digit, '.'+digit or
    // sign+'.'+digit.
    if first.is_ascii_digit() {
        return false;
    }
    let after_sign = if matches!(first, b'+' | b'-') { 1 } else { 0 };
    match b.get(after_sign) {
        Some(c) if c.is_ascii_digit() => return false,
        Some(b'.') if b.get(after_sign + 1).is_some_and(u8::is_ascii_digit) => return false,
        _ => {}
    }
    if matches!(s, "true" | "false" | "null" | "inf" | "-inf" | "nan") {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        if c >= 0x80 {
            return s[i..].chars().all(crate::parse::is_ident_char);
        }
        if !crate::parse::IDENT_ASCII[c as usize] {
            return false;
        }
    }
    true
}

fn write_quoted(out: &mut String, s: &str) {
    out.reserve(s.len() + 2);
    out.push('"');
    let bytes = s.as_bytes();
    let mut plain_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // ASCII that needs no escaping: the common case.
        if (0x20..0x7F).contains(&b) && b != b'"' && b != b'\\' {
            i += 1;
            continue;
        }
        let c = if b < 0x80 {
            b as char
        } else {
            s[i..].chars().next().unwrap()
        };
        let len = c.len_utf8();
        let esc: Option<&str> = match c {
            '"' => Some("\\\""),
            '\\' => Some("\\\\"),
            '\n' => Some("\\n"),
            '\r' => Some("\\r"),
            '\t' => Some("\\t"),
            '\u{08}' => Some("\\b"),
            '\u{0C}' => Some("\\f"),
            _ => None,
        };
        let unicode_escape =
            esc.is_none() && (crate::parse::is_disallowed(c) || crate::parse::is_newline_char(c));
        if esc.is_some() || unicode_escape {
            out.push_str(&s[plain_start..i]);
            match esc {
                Some(e) => out.push_str(e),
                None => {
                    let _ = write!(out, "\\u{{{:x}}}", c as u32);
                }
            }
            plain_start = i + len;
        }
        i += len;
    }
    out.push_str(&s[plain_start..]);
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn value_str(v: Value<'_>) -> String {
        let mut s = String::new();
        write_value(&mut s, &v);
        s
    }

    #[test]
    fn strings_matches_kdl_rs_style() {
        let cases = [
            ("api", "api"),
            (
                "surrealdb/surrealdb:v2.4.0",
                "\"surrealdb/surrealdb:v2.4.0\"",
            ),
            ("", "\"\""),
            ("a b", "\"a b\""),
            ("true", "\"true\""),
            ("1abc", "\"1abc\""),
            ("-1", "\"-1\""),
            (".5", "\".5\""),
            ("-.x", "-.x"),
            ("q\"\\\n\t", "\"q\\\"\\\\\\n\\t\""),
            ("bell\u{7}", "\"bell\\u{7}\""),
        ];
        for (input, expected) in cases {
            assert_eq!(value_str(Value::String(Cow::Borrowed(input))), expected);
        }
    }

    #[test]
    fn integers() {
        for i in [0i128, 7, -7, 1234567890, i128::MAX, i128::MIN] {
            assert_eq!(value_str(Value::Integer(i)), i.to_string());
        }
    }

    #[test]
    fn floats() {
        assert_eq!(value_str(Value::Float(1.0)), "1.0");
        assert_eq!(value_str(Value::Float(1e21)), "1e21");
        assert_eq!(value_str(Value::Float(1e-7)), "1e-7");
        assert_eq!(value_str(Value::Float(f64::NEG_INFINITY)), "#-inf");
        assert_eq!(value_str(Value::Float(f64::NAN)), "#nan");
    }

    #[test]
    fn nested_layout() {
        let mut port = Node::new("port");
        port.entries_mut()
            .push(Entry::prop("host", Value::Integer(80)));
        let mut inner = Node::new("inner");
        inner.entries_mut().push(Entry::arg(Value::Float(1.0)));
        port.children_mut().push(inner);
        let mut svc = Node::new("service");
        svc.entries_mut()
            .push(Entry::arg(Value::String(Cow::Borrowed("api"))));
        svc.children_mut().push(port);
        let mut out = String::new();
        write_document(
            &mut out,
            &Document::from_nodes(vec![svc, Node::new("last")]),
        );
        assert_eq!(
            out,
            "service api {\n    port host=80 {\n        inner 1.0\n    }\n}\nlast\n"
        );
    }
}
