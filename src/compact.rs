//! Opt-in rule-based name aliases, encoded using ordinary KDL v2.
//!
//! Enable the `compact` feature. Normal [`crate::to_string`] and
//! [`crate::from_str`] continue to use original names. These APIs are for
//! machine-oriented storage and messaging, not human-edited configuration.
//! Compact output omits structural indentation while keeping one node per line.
//! Braces preserve parent/child relationships; whitespace inside values is kept.
//!
//! ```
//! use club_kdl::{KdlDeserialize, KdlSerialize, compact};
//!
//! #[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
//! #[kdl(name = "position_update")]
//! struct Update {
//!     #[kdl(property)]
//!     object_identifier: u64,
//! }
//! let value = Update { object_identifier: 42 };
//! // Self-contained storage: includes the name table.
//! let file = compact::to_string(&value)?;
//! assert_eq!(compact::from_str::<Update>(&file)?, value);
//! // Messaging: exchange the table once, then reuse it.
//! let sender = compact::NameTable::from_value(&value)?;
//! let receiver = compact::NameTable::from_kdl(&sender.to_kdl())?;
//! let message = sender.encode(&value)?;
//! assert_eq!(receiver.decode::<Update>(&message)?, value);
//! # Ok::<(), compact::CompactError>(())
//! ```
//!
//! Tables declare the **complete vocabulary**, including unchanged names. A
//! sample value may omit optional children or dynamic map keys: use
//! [`NameTable::from_names`] with their union in that case. Unknown names are
//! errors; create and exchange a new table when the vocabulary changes.
//!
//! Sizes include metadata and may increase for small inputs. Encoding builds a
//! tree and performs lookups; this API does not guarantee faster serialization.
//! The table digest detects mismatched dictionaries, not tampering with values.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use sha2::{Digest, Sha256};

use crate::{Document, Entry, KdlDeserialize, KdlSerialize, Node, Value};

const TABLE_NODE: &str = "kdl-names";
const MESSAGE_NODE: &str = "kdl-message";
const VERSION: i128 = 1;
const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// An error in a compact table, envelope, or KDL payload.
#[derive(Debug, thiserror::Error)]
pub enum CompactError {
    /// The underlying KDL operation failed.
    #[error(transparent)]
    Kdl(#[from] crate::Error),
    /// This format version is not supported.
    #[error("unsupported compact format version: {0}")]
    UnsupportedVersion(i128),
    /// A name table is malformed or ambiguous.
    #[error("invalid compact name table: {0}")]
    InvalidTable(String),
    /// A message header is malformed.
    #[error("invalid compact envelope: {0}")]
    InvalidEnvelope(String),
    /// A message uses another name table.
    #[error("compact table mismatch: expected {expected}, found {found}")]
    TableMismatch {
        /// The receiving table's identifier.
        expected: String,
        /// The identifier carried by the message.
        found: String,
    },
    /// An original name was not declared in the table.
    #[error("name not in compact table: {0}")]
    UnknownName(String),
    /// A wire name was not declared in the table.
    #[error("unknown compact alias: {0}")]
    UnknownAlias(String),
}

/// Result of a compact codec operation.
pub type Result<T> = std::result::Result<T, CompactError>;

/// An immutable bidirectional vocabulary for node and property names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTable {
    // Ordered by original name, also the canonical export order.
    encode: BTreeMap<String, String>,
    decode: BTreeMap<String, String>,
    id: String,
}

impl NameTable {
    /// Build a deterministic table from a complete vocabulary.
    /// Names are deduplicated and sorted. Aliases are ASCII letters, shortest
    /// first, and never shadow any original name. A name changes only if its
    /// serialized KDL spelling is at least three bytes and the alias is smaller.
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names: BTreeSet<String> = names.into_iter().map(|s| s.as_ref().to_owned()).collect();
        let mut encode = BTreeMap::new();
        let mut alias = vec![b'a'];
        for name in &names {
            loop {
                let candidate = std::str::from_utf8(&alias).expect("ASCII alias");
                let mut spelling = String::new();
                crate::write::write_string(&mut spelling, candidate);
                // KDL keywords such as inf/nan need quotes, so they are not
                // useful aliases at their apparent byte length.
                if !names.contains(candidate) && spelling.len() == candidate.len() {
                    break;
                }
                advance_alias(&mut alias);
            }
            let mut spelling = String::new();
            crate::write::write_string(&mut spelling, name);
            let wire_name = if spelling.len() >= 3 && alias.len() < spelling.len() {
                let result = String::from_utf8(alias.clone()).expect("ASCII alias");
                advance_alias(&mut alias);
                result
            } else {
                name.clone()
            };
            encode.insert(name.clone(), wire_name);
        }
        Self::from_mapping(encode)
    }
    /// Collect node and property names from a document.
    pub fn from_document(doc: &Document<'_>) -> Self {
        fn collect(nodes: &[Node<'_>], names: &mut BTreeSet<String>) {
            for node in nodes {
                names.insert(node.name().to_owned());
                for entry in node.entries() {
                    if let Some(name) = entry.name() {
                        names.insert(name.to_owned());
                    }
                }
                collect(node.children(), names);
            }
        }
        let mut names = BTreeSet::new();
        collect(doc.nodes(), &mut names);
        Self::from_names(names)
    }
    /// Collect node and property names from an object.
    pub fn from_value<T: KdlSerialize + ?Sized>(value: &T) -> Result<Self> {
        Ok(Self::from_document(&value.to_kdl_doc()?))
    }
    /// Stable SHA-256 identity of the ordered name pairs.
    /// This identifies the dictionary, not the application schema, and is not
    /// an authentication tag. Versioned, length-prefixed UTF-8 pairs make the
    /// digest independent of the KDL writer's whitespace and quoting choices.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Export the table as KDL without indentation.
    pub fn to_kdl(&self) -> String {
        let mut node = version_node(TABLE_NODE);
        for (original, alias) in &self.encode {
            let mut entry = Node::new(alias.as_str());
            entry
                .entries
                .push(Entry::arg(Value::String(Cow::Borrowed(original))));
            node.children.push(entry);
        }
        let mut out = String::new();
        crate::write::write_node_unindented(&mut out, &node);
        out
    }
    /// Import a table from KDL.
    /// The input must contain exactly one table, with unique original and wire
    /// names. Imported aliases may be arbitrary KDL strings, including numeric
    /// strings; aliases shadowing a different original name are rejected.
    pub fn from_kdl(input: &str) -> Result<Self> {
        let doc = Document::parse(input)?;
        if doc.nodes().len() != 1 {
            return Err(CompactError::InvalidTable(
                "expected exactly one table node".into(),
            ));
        }
        Self::from_node(&doc.nodes()[0])
    }
    /// Serialize an object using this shared table.
    pub fn encode<T: KdlSerialize + ?Sized>(&self, value: &T) -> Result<String> {
        self.encode_document(value.to_kdl_doc()?)
    }
    /// Deserialize an object using this shared table.
    pub fn decode<T: for<'de> KdlDeserialize<'de>>(&self, input: &str) -> Result<T> {
        Ok(T::from_kdl_doc(&self.decode_document(input)?)?)
    }
    /// Encode a document, consuming its temporary tree.
    pub fn encode_document(&self, doc: Document<'_>) -> Result<String> {
        let doc = self.remap(doc, false)?;
        let mut header = version_node(MESSAGE_NODE);
        header
            .entries
            .push(Entry::arg(Value::String(Cow::Borrowed(&self.id))));
        let mut out = header.to_string();
        crate::write::write_document_unindented(&mut out, &doc);
        Ok(out)
    }
    /// Decode a document, borrowing names from this table and values from input.
    pub fn decode_document<'a>(&'a self, input: &'a str) -> Result<Document<'a>> {
        let mut nodes = Document::parse(input)?.into_nodes();
        let header = take_header(&mut nodes)?;
        if header.name() != MESSAGE_NODE
            || header.ty().is_some()
            || header.entries().len() != 2
            || !header.children().is_empty()
            || !header.entries().iter().all(is_untyped_argument)
        {
            return Err(CompactError::InvalidEnvelope(
                "expected kdl-message version table-id".into(),
            ));
        }
        check_version(&header)?;
        let id = header
            .arg(1)
            .and_then(Value::as_string)
            .ok_or_else(|| CompactError::InvalidEnvelope("table ID must be a string".into()))?;
        if id != self.id {
            return Err(CompactError::TableMismatch {
                expected: self.id.clone(),
                found: id.to_owned(),
            });
        }
        self.remap(Document::from_nodes(nodes), true)
    }

    fn from_mapping(encode: BTreeMap<String, String>) -> Self {
        let decode = encode.iter().map(|(k, v)| (v.clone(), k.clone())).collect();
        let mut table = Self {
            encode,
            decode,
            id: String::new(),
        };
        let mut digest = Sha256::new();
        digest.update(b"club-kdl:compact:1\0");
        for (original, alias) in &table.encode {
            for name in [original, alias] {
                digest.update((name.len() as u64).to_be_bytes());
                digest.update(name.as_bytes());
            }
        }
        for byte in digest.finalize() {
            write!(&mut table.id, "{byte:02x}").expect("writing to String");
        }
        table
    }

    fn from_node(node: &Node<'_>) -> Result<Self> {
        if node.name() != TABLE_NODE
            || node.ty().is_some()
            || node.entries().len() != 1
            || !node.entries().iter().all(is_untyped_argument)
        {
            return Err(CompactError::InvalidTable(
                "expected kdl-names version".into(),
            ));
        }
        check_version(node)?;
        let mut encode = BTreeMap::new();
        let mut aliases = BTreeSet::new();
        for entry in node.children() {
            if entry.ty().is_some()
                || !entry.children().is_empty()
                || entry.entries().len() != 1
                || !entry.entries().iter().all(is_untyped_argument)
            {
                return Err(CompactError::InvalidTable(
                    "expected alias followed by one original name".into(),
                ));
            }
            let original = entry.arg(0).and_then(Value::as_string).ok_or_else(|| {
                CompactError::InvalidTable("original name must be a string".into())
            })?;
            if encode
                .insert(original.to_owned(), entry.name().to_owned())
                .is_some()
                || !aliases.insert(entry.name().to_owned())
            {
                return Err(CompactError::InvalidTable(
                    "duplicate original name or alias".into(),
                ));
            }
        }
        for (original, alias) in &encode {
            if original != alias && encode.contains_key(alias) {
                return Err(CompactError::InvalidTable(format!(
                    "alias shadows original name: {alias}"
                )));
            }
        }
        Ok(Self::from_mapping(encode))
    }

    fn remap<'a>(&'a self, mut doc: Document<'a>, decode: bool) -> Result<Document<'a>> {
        fn visit<'a>(
            nodes: &mut [Node<'a>],
            map: &'a BTreeMap<String, String>,
            decode: bool,
        ) -> Result<()> {
            fn lookup<'a>(
                map: &'a BTreeMap<String, String>,
                name: &str,
                decode: bool,
            ) -> Result<&'a str> {
                map.get(name).map(String::as_str).ok_or_else(|| {
                    if decode {
                        CompactError::UnknownAlias(name.to_owned())
                    } else {
                        CompactError::UnknownName(name.to_owned())
                    }
                })
            }
            for node in nodes {
                node.name = Cow::Borrowed(lookup(map, node.name(), decode)?);
                for entry in &mut node.entries {
                    if let Some(name) = &entry.name {
                        entry.name = Some(Cow::Borrowed(lookup(map, name, decode)?));
                    }
                }
                visit(&mut node.children, map, decode)?;
            }
            Ok(())
        }
        let map = if decode { &self.decode } else { &self.encode };
        visit(&mut doc.root.children, map, decode)?;
        Ok(doc)
    }
}

/// Serialize an object to a self-contained compact KDL file.
pub fn to_string<T: KdlSerialize + ?Sized>(value: &T) -> Result<String> {
    encode_document(value.to_kdl_doc()?)
}

/// Deserialize an object from a self-contained compact KDL file.
pub fn from_str<T: for<'de> KdlDeserialize<'de>>(input: &str) -> Result<T> {
    let mut nodes = Document::parse(input)?.into_nodes();
    let table = NameTable::from_node(&take_header(&mut nodes)?)?;
    let doc = table.remap(Document::from_nodes(nodes), true)?;
    Ok(T::from_kdl_doc(&doc)?)
}

/// Encode a document with an embedded name table.
pub fn encode_document(doc: Document<'_>) -> Result<String> {
    let table = NameTable::from_document(&doc);
    let doc = table.remap(doc, false)?;
    let mut out = table.to_kdl();
    crate::write::write_document_unindented(&mut out, &doc);
    Ok(out)
}

/// Restore a self-contained file to a document with owned original names/values.
pub fn decode_document(input: &str) -> Result<Document<'static>> {
    let mut nodes = Document::parse(input)?.into_nodes();
    let table = NameTable::from_node(&take_header(&mut nodes)?)?;
    Ok(table.remap(Document::from_nodes(nodes), true)?.into_owned())
}

fn advance_alias(alias: &mut Vec<u8>) {
    for byte in alias.iter_mut().rev() {
        let index = ALPHABET
            .iter()
            .position(|b| b == byte)
            .expect("ASCII alias");
        if index + 1 < ALPHABET.len() {
            *byte = ALPHABET[index + 1];
            return;
        }
        *byte = ALPHABET[0];
    }
    alias.insert(0, ALPHABET[0]);
}

fn version_node(name: &str) -> Node<'_> {
    let mut node = Node::new(name);
    node.entries.push(Entry::arg(Value::Integer(VERSION)));
    node
}

fn is_untyped_argument(entry: &Entry<'_>) -> bool {
    entry.name().is_none() && entry.ty().is_none()
}

fn check_version(node: &Node<'_>) -> Result<()> {
    match node.arg(0) {
        Some(Value::Integer(VERSION)) => Ok(()),
        Some(Value::Integer(v)) => Err(CompactError::UnsupportedVersion(*v)),
        _ => Err(CompactError::InvalidEnvelope(
            "version must be an integer".into(),
        )),
    }
}

fn take_header<'a>(nodes: &mut Vec<Node<'a>>) -> Result<Node<'a>> {
    if nodes.is_empty() {
        Err(CompactError::InvalidEnvelope("missing header".into()))
    } else {
        Ok(nodes.remove(0))
    }
}
