//! Hand-written KDL v2 parser producing the lightweight [`crate::ast`] tree.
//!
//! The grammar follows the KDL 2.0.0 spec. The parser works on bytes with an
//! ASCII fast path and only decodes UTF-8 when it meets a non-ASCII byte.
//! Strings that need no unescaping (identifiers, plain quoted strings,
//! single-line raw strings) are borrowed from the source.

use std::borrow::Cow;

use crate::ast::{Document, Entry, Node, Value};
use crate::error::{Error, Result};

/// Internal result type. Boxing the error keeps `Result<bool>` and friends
/// small, which matters on the parser's hot paths.
type PResult<T> = std::result::Result<T, Box<Error>>;

/// Maximum nesting depth of children blocks. Guards against stack overflow on
/// adversarial input.
const MAX_DEPTH: usize = 256;

pub(crate) fn parse_document(src: &str) -> Result<Document<'_>> {
    let mut p = Parser {
        src,
        b: src.as_bytes(),
        pos: 0,
        depth: 0,
    };
    if src.starts_with('\u{FEFF}') {
        p.pos = 3;
    }
    let nodes = p.nodes(false).map_err(|e| *e)?;
    Ok(Document::from_nodes(nodes))
}

struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    depth: usize,
}

/// ASCII bytes that may appear in an identifier string.
const fn ident_table() -> [bool; 128] {
    let mut t = [false; 128];
    let mut i = 0x21;
    while i < 0x7F {
        t[i] = true;
        i += 1;
    }
    let banned = b"\\/(){};[]\"#=";
    let mut j = 0;
    while j < banned.len() {
        t[banned[j] as usize] = false;
        j += 1;
    }
    t
}
pub(crate) static IDENT_ASCII: [bool; 128] = ident_table();

#[inline]
pub(crate) fn is_unicode_space(c: char) -> bool {
    matches!(
        c,
        '\t' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

#[inline]
pub(crate) fn is_newline_char(c: char) -> bool {
    matches!(
        c,
        '\r' | '\n' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

#[inline]
pub(crate) fn is_disallowed(c: char) -> bool {
    matches!(
        c,
        '\u{0000}'..='\u{0008}'
            | '\u{000E}'..='\u{001F}'
            | '\u{007F}'
            | '\u{200E}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

#[inline]
pub(crate) fn is_ident_char(c: char) -> bool {
    if c.is_ascii() {
        IDENT_ASCII[c as usize]
    } else {
        !is_unicode_space(c) && !is_newline_char(c) && !is_disallowed(c)
    }
}

impl<'a> Parser<'a> {
    // ------------------------------------------------------------------
    // Low-level helpers
    // ------------------------------------------------------------------

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    #[inline]
    fn peek_at(&self, off: usize) -> Option<u8> {
        self.b.get(self.pos + off).copied()
    }

    #[inline]
    fn starts_with(&self, s: &[u8]) -> bool {
        self.b[self.pos..].starts_with(s)
    }

    #[inline]
    fn eof(&self) -> bool {
        self.pos >= self.b.len()
    }

    #[inline]
    fn peek_char(&self) -> Option<char> {
        match self.peek() {
            None => None,
            Some(b) if b < 0x80 => Some(b as char),
            Some(_) => self.src[self.pos..].chars().next(),
        }
    }

    #[cold]
    #[inline(never)]
    fn err<T>(&self, msg: impl std::fmt::Display) -> PResult<T> {
        Err(self.error_at(self.pos, msg))
    }

    #[cold]
    #[inline(never)]
    fn error_at(&self, pos: usize, msg: impl std::fmt::Display) -> Box<Error> {
        let pos = pos.min(self.src.len());
        let before = &self.src[..pos];
        let line = before.matches('\n').count() + 1;
        let col = before
            .rfind('\n')
            .map_or(before, |i| &before[i + 1..])
            .chars()
            .count()
            + 1;
        Box::new(Error::Parse(format!("line {line}, column {col}: {msg}")))
    }

    /// Consume one newline sequence if present.
    #[inline]
    fn newline(&mut self) -> bool {
        match self.peek() {
            Some(b'\n') | Some(0x0B) | Some(0x0C) => {
                self.pos += 1;
                true
            }
            Some(b'\r') => {
                self.pos += if self.peek_at(1) == Some(b'\n') { 2 } else { 1 };
                true
            }
            Some(b) if b >= 0x80 => match self.peek_char() {
                Some(c) if is_newline_char(c) => {
                    self.pos += c.len_utf8();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    #[inline]
    fn at_newline(&self) -> bool {
        match self.peek() {
            Some(b'\n' | b'\r' | 0x0B | 0x0C) => true,
            Some(b) if b >= 0x80 => self.peek_char().is_some_and(is_newline_char),
            _ => false,
        }
    }

    /// Consume one unicode-space character if present.
    #[inline]
    fn unicode_space(&mut self) -> bool {
        match self.peek() {
            Some(b' ' | b'\t') => {
                self.pos += 1;
                true
            }
            Some(b) if b >= 0x80 => match self.peek_char() {
                Some(c) if is_unicode_space(c) => {
                    self.pos += c.len_utf8();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// `'/*' commented-block` (nestable).
    fn multi_line_comment(&mut self) -> PResult<bool> {
        if !self.starts_with(b"/*") {
            return Ok(false);
        }
        let start = self.pos;
        self.pos += 2;
        let mut depth = 1usize;
        while depth > 0 {
            match self.peek() {
                None => return Err(self.error_at(start, "unterminated block comment")),
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    depth -= 1;
                    self.pos += 2;
                }
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    depth += 1;
                    self.pos += 2;
                }
                Some(_) => self.advance_char_checked()?,
            }
        }
        Ok(true)
    }

    /// Advance past one character, rejecting disallowed code points.
    #[inline]
    fn advance_char_checked(&mut self) -> PResult<()> {
        let b = self.b[self.pos];
        if b < 0x80 {
            if is_disallowed(b as char) {
                return self.err(format!("disallowed character U+{b:04X}"));
            }
            self.pos += 1;
        } else {
            let c = self.peek_char().unwrap();
            if is_disallowed(c) {
                return self.err(format!("disallowed character U+{:04X}", c as u32));
            }
            self.pos += c.len_utf8();
        }
        Ok(())
    }

    /// `ws := unicode-space | multi-line-comment`
    #[inline]
    fn ws(&mut self) -> PResult<bool> {
        if self.unicode_space() {
            return Ok(true);
        }
        self.multi_line_comment()
    }

    /// `'//' ^newline* (newline | eof)`
    fn single_line_comment(&mut self) -> PResult<bool> {
        if !self.starts_with(b"//") {
            return Ok(false);
        }
        self.pos += 2;
        loop {
            match self.peek() {
                Some(b) if (0x20..0x7F).contains(&b) || b == b'\t' => self.pos += 1,
                None => break,
                _ if self.at_newline() => break,
                _ => self.advance_char_checked()?,
            }
        }
        self.newline();
        Ok(true)
    }

    /// `escline := '\\' ws* (single-line-comment | newline | eof)`
    fn escline(&mut self) -> PResult<bool> {
        if self.peek() != Some(b'\\') {
            return Ok(false);
        }
        let save = self.pos;
        self.pos += 1;
        while self.ws()? {}
        if self.single_line_comment()? || self.newline() || self.eof() {
            Ok(true)
        } else {
            self.pos = save;
            Ok(false)
        }
    }

    /// `node-space*`; returns whether anything was consumed.
    fn node_space(&mut self) -> PResult<bool> {
        let start = self.pos;
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t')) {
                self.pos += 1;
            }
            if self.ws()? || self.escline()? {
                continue;
            }
            break;
        }
        Ok(self.pos != start)
    }

    /// `line-space*`
    fn line_space(&mut self) -> PResult<()> {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
            if self.newline() || self.node_space()? || self.single_line_comment()? {
                continue;
            }
            return Ok(());
        }
    }

    // ------------------------------------------------------------------
    // Nodes
    // ------------------------------------------------------------------

    fn nodes(&mut self, in_children: bool) -> PResult<Vec<Node<'a>>> {
        let mut nodes = Vec::new();
        loop {
            self.line_space()?;
            match self.peek() {
                None => {
                    if in_children {
                        return self.err("unterminated children block, expected '}'");
                    }
                    return Ok(nodes);
                }
                Some(b'}') => {
                    if in_children {
                        return Ok(nodes);
                    }
                    return self.err("unexpected '}'");
                }
                _ => {}
            }
            if self.starts_with(b"/-") {
                self.pos += 2;
                self.line_space()?;
                self.node(in_children)?;
            } else {
                let node = self.node(in_children)?;
                nodes.push(node);
            }
        }
    }

    fn node(&mut self, in_children: bool) -> PResult<Node<'a>> {
        let ty = if self.peek() == Some(b'(') {
            let t = self.type_annotation()?;
            self.node_space()?;
            Some(t)
        } else {
            None
        };
        let name = match self.string()? {
            Some(s) => s,
            None => return self.err("expected a node name"),
        };
        let mut node = Node {
            name,
            ty,
            entries: Vec::new(),
            children: Vec::new(),
        };

        // Once any children block (slashdashed or not) appears, no more
        // entries are allowed; once a real one appears, only slashdashed
        // blocks may follow.
        let mut entries_closed = false;
        let mut has_children = false;
        loop {
            let spaced = self.node_space()?;
            match self.peek() {
                None | Some(b';') | Some(b'}') => break,
                _ if self.at_newline() || self.starts_with(b"//") => break,
                Some(b'{') => {
                    if has_children {
                        return self.err("a node may only have one children block");
                    }
                    node.children = self.children_block()?;
                    has_children = true;
                    entries_closed = true;
                }
                _ if self.starts_with(b"/-") => {
                    self.pos += 2;
                    self.line_space()?;
                    if self.peek() == Some(b'{') {
                        self.children_block()?;
                        entries_closed = true;
                    } else {
                        if entries_closed {
                            return self.err("entries must come before children blocks");
                        }
                        if !spaced {
                            return self.err("expected whitespace before entry");
                        }
                        self.entry()?;
                    }
                }
                _ => {
                    if entries_closed {
                        return self.err("entries must come before children blocks");
                    }
                    if !spaced {
                        return self.err("expected whitespace before entry");
                    }
                    let e = self.entry()?;
                    node.entries.push(e);
                }
            }
        }

        // node-terminator
        match self.peek() {
            None => {}
            Some(b';') => self.pos += 1,
            Some(b'}') => {
                if !in_children {
                    return self.err("unexpected '}'");
                }
            }
            _ => {
                if !self.newline() && !self.single_line_comment()? {
                    return self.err("expected node terminator");
                }
            }
        }
        Ok(node)
    }

    fn children_block(&mut self) -> PResult<Vec<Node<'a>>> {
        debug_assert_eq!(self.peek(), Some(b'{'));
        if self.depth >= MAX_DEPTH {
            return self.err("children nested too deeply");
        }
        self.pos += 1;
        self.depth += 1;
        let nodes = self.nodes(true)?;
        self.depth -= 1;
        // nodes(true) only returns Ok when positioned at '}'
        self.pos += 1;
        Ok(nodes)
    }

    /// `type := '(' node-space* string node-space* ')'`
    fn type_annotation(&mut self) -> PResult<Cow<'a, str>> {
        self.pos += 1; // '('
        self.node_space()?;
        let s = match self.string()? {
            Some(s) => s,
            None => return self.err("expected a type name"),
        };
        self.node_space()?;
        if self.peek() != Some(b')') {
            return self.err("expected ')'");
        }
        self.pos += 1;
        Ok(s)
    }

    /// `prop | value`
    fn entry(&mut self) -> PResult<Entry<'a>> {
        if self.peek() == Some(b'(') {
            let ty = self.type_annotation()?;
            self.node_space()?;
            let value = self.value_body()?;
            return Ok(Entry {
                name: None,
                ty: Some(ty),
                value,
            });
        }
        if let Some(s) = self.string()? {
            let after = self.pos;
            self.node_space()?;
            if self.peek() == Some(b'=') {
                self.pos += 1;
                self.node_space()?;
                let ty = if self.peek() == Some(b'(') {
                    let t = self.type_annotation()?;
                    self.node_space()?;
                    Some(t)
                } else {
                    None
                };
                let value = self.value_body()?;
                return Ok(Entry {
                    name: Some(s),
                    ty,
                    value,
                });
            }
            self.pos = after;
            return Ok(Entry {
                name: None,
                ty: None,
                value: Value::String(s),
            });
        }
        let value = self.value_body()?;
        Ok(Entry {
            name: None,
            ty: None,
            value,
        })
    }

    /// `string | number | keyword`
    fn value_body(&mut self) -> PResult<Value<'a>> {
        if let Some(s) = self.string()? {
            return Ok(Value::String(s));
        }
        match self.peek() {
            Some(b'#') => self.keyword(),
            Some(b'0'..=b'9') => self.number(),
            Some(b'+' | b'-') if matches!(self.peek_at(1), Some(b'0'..=b'9')) => self.number(),
            _ => self.err("expected a value"),
        }
    }

    fn keyword(&mut self) -> PResult<Value<'a>> {
        const KEYWORDS: [(&[u8], Value<'static>); 6] = [
            (b"#true", Value::Bool(true)),
            (b"#false", Value::Bool(false)),
            (b"#null", Value::Null),
            (b"#inf", Value::Float(f64::INFINITY)),
            (b"#-inf", Value::Float(f64::NEG_INFINITY)),
            (b"#nan", Value::Float(f64::NAN)),
        ];
        for (kw, v) in KEYWORDS {
            if self.starts_with(kw) {
                let save = self.pos;
                self.pos += kw.len();
                if self.peek_char().is_some_and(is_ident_char) {
                    self.pos = save;
                    break;
                }
                return Ok(v);
            }
        }
        self.err("unknown keyword")
    }

    // ------------------------------------------------------------------
    // Numbers
    // ------------------------------------------------------------------

    fn number(&mut self) -> PResult<Value<'a>> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        let radix = match (self.peek(), self.peek_at(1)) {
            (Some(b'0'), Some(b'x')) => 16,
            (Some(b'0'), Some(b'o')) => 8,
            (Some(b'0'), Some(b'b')) => 2,
            _ => 10,
        };
        let value = if radix != 10 {
            self.pos += 2;
            let digits_start = self.pos;
            let is_digit = |b: u8| match radix {
                16 => b.is_ascii_hexdigit(),
                8 => (b'0'..=b'7').contains(&b),
                _ => b == b'0' || b == b'1',
            };
            if !self.peek().is_some_and(is_digit) {
                return self.err("expected digit after radix prefix");
            }
            while self.peek().is_some_and(|b| is_digit(b) || b == b'_') {
                self.pos += 1;
            }
            let digits = strip_underscores(&self.src[digits_start..self.pos]);
            let negative = self.b[start] == b'-';
            let magnitude = u128::from_str_radix(&digits, radix)
                .map_err(|_| self.error_at(start, "integer out of range"))?;
            Value::Integer(
                to_i128(magnitude, negative)
                    .ok_or_else(|| self.error_at(start, "integer out of range"))?,
            )
        } else {
            self.digits()?;
            let mut is_float = false;
            if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
                self.digits()?;
                is_float = true;
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                let save = self.pos;
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.digits()?;
                    is_float = true;
                } else {
                    self.pos = save;
                }
            }
            let text = strip_underscores(&self.src[start..self.pos]);
            if is_float {
                Value::Float(
                    text.parse::<f64>()
                        .map_err(|_| self.error_at(start, "invalid float"))?,
                )
            } else {
                Value::Integer(
                    text.parse::<i128>()
                        .map_err(|_| self.error_at(start, "integer out of range"))?,
                )
            }
        };
        if self.peek_char().is_some_and(is_ident_char) {
            return self.err("invalid character in number");
        }
        Ok(value)
    }

    /// `digit (digit | '_')*`
    fn digits(&mut self) -> PResult<()> {
        if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
            return self.err("expected digit");
        }
        while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'_') {
            self.pos += 1;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Strings
    // ------------------------------------------------------------------

    /// Parse any string form, or return `None` if the input does not start
    /// with one.
    fn string(&mut self) -> PResult<Option<Cow<'a, str>>> {
        match self.peek() {
            Some(b'"') => self.quoted_string().map(Some),
            Some(b'#') => {
                let mut i = 1;
                while self.peek_at(i) == Some(b'#') {
                    i += 1;
                }
                if self.peek_at(i) == Some(b'"') {
                    self.raw_string().map(Some)
                } else {
                    Ok(None)
                }
            }
            Some(_) => self.identifier(),
            None => Ok(None),
        }
    }

    fn identifier(&mut self) -> PResult<Option<Cow<'a, str>>> {
        let start = self.pos;
        let first = match self.peek_char() {
            Some(c) if is_ident_char(c) => c,
            _ => return Ok(None),
        };
        if first.is_ascii_digit() {
            return Ok(None);
        }
        // sign followed by digit is a number
        if (first == '+' || first == '-') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        loop {
            match self.peek() {
                Some(b) if b < 0x80 => {
                    if IDENT_ASCII[b as usize] {
                        self.pos += 1;
                    } else if is_disallowed(b as char) {
                        return self.err(format!("disallowed character U+{b:04X}"));
                    } else {
                        break;
                    }
                }
                Some(_) => {
                    let c = self.peek_char().unwrap();
                    if is_ident_char(c) {
                        self.pos += c.len_utf8();
                    } else if is_disallowed(c) {
                        return self.err(format!("disallowed character U+{:04X}", c as u32));
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
        let s = &self.src[start..self.pos];
        let bytes = s.as_bytes();
        // dotted-ident: `.` (optionally after a sign) must not be followed
        // by a digit.
        let after_sign = if matches!(bytes[0], b'+' | b'-') {
            1
        } else {
            0
        };
        if bytes.get(after_sign) == Some(&b'.')
            && bytes
                .get(after_sign + 1)
                .is_some_and(|b| b.is_ascii_digit())
        {
            return Err(self.error_at(start, "identifier cannot look like a number"));
        }
        if matches!(s, "true" | "false" | "null" | "inf" | "-inf" | "nan") {
            return Err(self.error_at(
                start,
                format!("bare `{s}` is not allowed; use `#{s}` or quote it"),
            ));
        }
        Ok(Some(Cow::Borrowed(s)))
    }

    fn quoted_string(&mut self) -> PResult<Cow<'a, str>> {
        let start = self.pos;
        if self.starts_with(b"\"\"\"") {
            return self.multi_line_quoted(start);
        }
        self.pos += 1;
        let body_start = self.pos;
        let mut has_escape = false;
        loop {
            match self.peek() {
                Some(b) if (0x20..0x7F).contains(&b) && b != b'"' && b != b'\\' => {
                    self.pos += 1;
                }
                None => return Err(self.error_at(start, "unterminated string")),
                Some(b'"') => break,
                Some(b'\\') => {
                    has_escape = true;
                    self.pos += 1;
                    if self.eof() {
                        return Err(self.error_at(start, "unterminated string"));
                    }
                    // A ws-escape swallows any run of whitespace and
                    // newlines; other escapes are validated in `unescape`.
                    let mut ws_escape = false;
                    while self.newline() || self.unicode_space() {
                        ws_escape = true;
                    }
                    if !ws_escape {
                        self.advance_char_checked()?;
                    }
                }
                _ => {
                    if self.at_newline() {
                        return self
                            .err("newline in single-line string; use \"\"\" for multi-line");
                    }
                    self.advance_char_checked()?;
                }
            }
        }
        let body = &self.src[body_start..self.pos];
        self.pos += 1; // closing quote
        if has_escape {
            unescape(body)
                .map(Cow::Owned)
                .map_err(|(off, msg)| self.error_at(body_start + off, msg))
        } else {
            Ok(Cow::Borrowed(body))
        }
    }

    fn multi_line_quoted(&mut self, start: usize) -> PResult<Cow<'a, str>> {
        self.pos += 3;
        if !self.newline() {
            return self.err("multi-line string must start with a newline after \"\"\"");
        }
        let body_start = self.pos;
        loop {
            match self.peek() {
                None => return Err(self.error_at(start, "unterminated multi-line string")),
                Some(b'"') if self.starts_with(b"\"\"\"") => break,
                Some(b'\\') => {
                    self.pos += 1;
                    if self.eof() {
                        return Err(self.error_at(start, "unterminated multi-line string"));
                    }
                    if !self.newline() {
                        self.advance_char_checked()?;
                    }
                }
                _ => {
                    if !self.newline() {
                        self.advance_char_checked()?;
                    }
                }
            }
        }
        let body = &self.src[body_start..self.pos];
        self.pos += 3;
        // Whitespace escapes are resolved before dedenting, the rest after.
        let joined = remove_ws_escapes(body);
        let dedented = dedent(&joined).map_err(|(_, msg)| self.error_at(body_start, msg))?;
        let out = unescape(&dedented).map_err(|(_, msg)| self.error_at(body_start, msg))?;
        Ok(Cow::Owned(out))
    }

    fn raw_string(&mut self) -> PResult<Cow<'a, str>> {
        let start = self.pos;
        let mut hashes = 0;
        while self.peek() == Some(b'#') {
            hashes += 1;
            self.pos += 1;
        }
        // closing delimiter: '"' followed by `hashes` '#'
        let is_close = |p: &Self, quotes: usize| {
            let mut i = 0;
            while i < quotes {
                if p.peek_at(i) != Some(b'"') {
                    return false;
                }
                i += 1;
            }
            (0..hashes).all(|h| p.peek_at(quotes + h) == Some(b'#'))
        };
        if self.starts_with(b"\"\"\"") {
            self.pos += 3;
            if !self.newline() {
                return self.err("multi-line raw string must start with a newline after \"\"\"");
            }
            let body_start = self.pos;
            loop {
                if self.eof() {
                    return Err(self.error_at(start, "unterminated raw string"));
                }
                if is_close(self, 3) {
                    break;
                }
                if !self.newline() {
                    self.advance_char_checked()?;
                }
            }
            let body = &self.src[body_start..self.pos];
            self.pos += 3 + hashes;
            let dedented =
                dedent(body).map_err(|(off, msg)| self.error_at(body_start + off, msg))?;
            return Ok(Cow::Owned(dedented));
        }
        self.pos += 1; // opening quote
        let body_start = self.pos;
        loop {
            if self.eof() {
                return Err(self.error_at(start, "unterminated raw string"));
            }
            if is_close(self, 1) {
                break;
            }
            if self.at_newline() {
                return self.err("newline in single-line raw string");
            }
            self.advance_char_checked()?;
        }
        let body = &self.src[body_start..self.pos];
        self.pos += 1 + hashes;
        Ok(Cow::Borrowed(body))
    }
}

fn strip_underscores(s: &str) -> Cow<'_, str> {
    if s.contains('_') {
        Cow::Owned(s.chars().filter(|&c| c != '_').collect())
    } else {
        Cow::Borrowed(s)
    }
}

fn to_i128(magnitude: u128, negative: bool) -> Option<i128> {
    if negative {
        if magnitude == 1u128 << 127 {
            Some(i128::MIN)
        } else {
            i128::try_from(magnitude).ok().map(|v| -v)
        }
    } else {
        i128::try_from(magnitude).ok()
    }
}

/// Split `s` into lines on any KDL newline, returning `(offset, line)`.
fn split_lines(s: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut iter = s.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if is_newline_char(c) {
            lines.push((line_start, &s[line_start..i]));
            let mut end = i + c.len_utf8();
            if c == '\r' && iter.peek().map(|&(_, n)| n) == Some('\n') {
                iter.next();
                end += 1;
            }
            line_start = end;
        }
    }
    lines.push((line_start, &s[line_start..]));
    lines
}

/// Dedent a multi-line string body. `body` runs from just after the opening
/// newline up to (not including) the closing `"""`; its last line is the
/// indentation prefix. Escapes are not interpreted here; callers resolve
/// whitespace escapes before and other escapes after.
fn dedent(body: &str) -> std::result::Result<String, (usize, String)> {
    let lines = split_lines(body);
    let (last_off, last) = *lines.last().unwrap();
    let prefix_ok = last.chars().all(is_unicode_space);
    if !prefix_ok {
        return Err((
            last_off,
            "closing \"\"\" of a multi-line string must be on its own line".into(),
        ));
    }
    let prefix = last;
    let content = &lines[..lines.len() - 1];
    let mut out = String::with_capacity(body.len());
    for (i, &(off, line)) in content.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.chars().all(is_unicode_space) {
            continue;
        }
        match line.strip_prefix(prefix) {
            Some(rest) => out.push_str(rest),
            None => {
                return Err((
                    off,
                    "multi-line string line does not match the closing indentation".into(),
                ));
            }
        }
    }
    Ok(out)
}

/// Remove whitespace escapes (`\\` followed by whitespace or newlines),
/// leaving every other escape untouched.
fn remove_ws_escapes(body: &str) -> Cow<'_, str> {
    if !body.contains('\\') {
        return Cow::Borrowed(body);
    }
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some(&n) if is_unicode_space(n) || is_newline_char(n) => {
                while chars
                    .peek()
                    .is_some_and(|&c| is_unicode_space(c) || is_newline_char(c))
                {
                    chars.next();
                }
            }
            Some(&n) => {
                out.push('\\');
                out.push(n);
                chars.next();
            }
            None => out.push('\\'),
        }
    }
    Cow::Owned(out)
}

/// Resolve escapes in a quoted string body.
fn unescape(body: &str) -> std::result::Result<String, (usize, String)> {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some((_, e)) = chars.next() else {
            return Err((i, "dangling backslash".into()));
        };
        match e {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'b' => out.push('\u{08}'),
            'f' => out.push('\u{0C}'),
            's' => out.push(' '),
            'u' => {
                if chars.next().map(|(_, c)| c) != Some('{') {
                    return Err((i, "expected '{' after \\u".into()));
                }
                let mut hex = String::new();
                loop {
                    match chars.next() {
                        Some((_, '}')) => break,
                        Some((_, h)) if h.is_ascii_hexdigit() && hex.len() < 6 => hex.push(h),
                        _ => return Err((i, "invalid unicode escape".into())),
                    }
                }
                let ch = u32::from_str_radix(&hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| (i, "invalid unicode scalar value in escape".to_string()))?;
                out.push(ch);
            }
            c if is_unicode_space(c) || is_newline_char(c) => {
                while chars
                    .peek()
                    .is_some_and(|&(_, c)| is_unicode_space(c) || is_newline_char(c))
                {
                    chars.next();
                }
            }
            other => return Err((i, format!("invalid escape '\\{other}'"))),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Document<'_> {
        parse_document(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    #[test]
    fn borrows_plain_strings() {
        let doc = p(r#"node "plain" bare key="v""#);
        let n = &doc.nodes()[0];
        assert!(matches!(n.name, Cow::Borrowed(_)));
        assert!(matches!(
            n.arg(0),
            Some(Value::String(Cow::Borrowed("plain")))
        ));
        assert!(matches!(
            n.arg(1),
            Some(Value::String(Cow::Borrowed("bare")))
        ));
        assert!(matches!(
            n.prop("key"),
            Some(Value::String(Cow::Borrowed("v")))
        ));
    }

    #[test]
    fn numbers() {
        let doc = p("n 1 -2 0x1f 0o17 0b101 1_000 1.5 1e3 -0.5E-2");
        let args: Vec<_> = doc.nodes()[0].args().cloned().collect();
        assert_eq!(
            args,
            vec![
                Value::Integer(1),
                Value::Integer(-2),
                Value::Integer(31),
                Value::Integer(15),
                Value::Integer(5),
                Value::Integer(1000),
                Value::Float(1.5),
                Value::Float(1000.0),
                Value::Float(-0.005),
            ]
        );
    }

    #[test]
    fn rightmost_prop_wins() {
        let doc = p("n a=1 a=2");
        assert_eq!(doc.nodes()[0].prop("a"), Some(&Value::Integer(2)));
    }

    #[test]
    fn error_has_position() {
        let err = parse_document("a\nb true").unwrap_err().to_string();
        assert!(err.contains("line 2, column 3"), "{err}");
    }

    #[test]
    fn deep_nesting_is_rejected() {
        let src = "a {".repeat(MAX_DEPTH + 1) + &"}".repeat(MAX_DEPTH + 1);
        assert!(parse_document(&src).is_err());
    }
}
