//! KDL Deserializer implementation.

use std::iter::Peekable;

use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::Error;

/// Deserialize a type from a KDL string.
pub fn from_str<T>(s: &str) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let doc: KdlDocument = s.parse()?;
    from_document(&doc)
}

/// Deserialize from a KdlDocument.
pub fn from_document<T>(doc: &KdlDocument) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let nodes: Vec<&KdlNode> = doc.nodes().iter().collect();

    if nodes.len() != 1 {
        return Err(Error::ExpectedSingleNode(nodes.len()));
    }

    from_node(nodes[0])
}

/// Deserialize from a KdlNode.
pub fn from_node<T>(node: &KdlNode) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let deserializer = NodeDeserializer::new(node);
    T::deserialize(deserializer)
}

/// Deserializer for a KDL node.
struct NodeDeserializer<'de> {
    node: &'de KdlNode,
}

impl<'de> NodeDeserializer<'de> {
    fn new(node: &'de KdlNode) -> Self {
        Self { node }
    }
}

impl<'de> de::Deserializer<'de> for NodeDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Default to struct-like deserialization
        self.deserialize_map(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(entry) = self.node.entries().first() {
            if let Some(b) = entry.value().as_bool() {
                return visitor.visit_bool(b);
            }
        }
        Err(Error::TypeMismatch {
            expected: "bool".into(),
            got: "other".into(),
        })
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(entry) = self.node.entries().first() {
            if let Some(n) = entry.value().as_integer() {
                return visitor.visit_i64(n as i64);
            }
        }
        Err(Error::TypeMismatch {
            expected: "integer".into(),
            got: "other".into(),
        })
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(entry) = self.node.entries().first() {
            if let Some(n) = entry.value().as_integer() {
                return visitor.visit_u64(n as u64);
            }
        }
        Err(Error::TypeMismatch {
            expected: "unsigned integer".into(),
            got: "other".into(),
        })
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(entry) = self.node.entries().first() {
            if let Some(n) = entry.value().as_float() {
                return visitor.visit_f64(n);
            }
        }
        Err(Error::TypeMismatch {
            expected: "float".into(),
            got: "other".into(),
        })
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(entry) = self.node.entries().first() {
            if let Some(s) = entry.value().as_string() {
                return visitor.visit_str(s);
            }
        }
        Err(Error::TypeMismatch {
            expected: "string".into(),
            got: "other".into(),
        })
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Use child nodes as sequence elements
        if let Some(children) = self.node.children() {
            let nodes: Vec<&KdlNode> = children.nodes().iter().collect();
            visitor.visit_seq(SeqDeserializer::new(nodes.into_iter()))
        } else {
            visitor.visit_seq(SeqDeserializer::new(std::iter::empty()))
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(NodeMapAccess::new(self.node))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Use node name as variant
        visitor.visit_enum(EnumDeserializer::new(self.node))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.node.name().value())
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// MapAccess for node properties.
struct NodeMapAccess<'de> {
    entries: Peekable<std::slice::Iter<'de, KdlEntry>>,
}

impl<'de> NodeMapAccess<'de> {
    fn new(node: &'de KdlNode) -> Self {
        Self {
            entries: node.entries().iter().peekable(),
        }
    }
}

impl<'de> MapAccess<'de> for NodeMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        // Find next property (entry with a name)
        while let Some(entry) = self.entries.peek() {
            if entry.name().is_some() {
                let name = entry.name().unwrap().value();
                return seed.deserialize(StrDeserializer(name)).map(Some);
            }
            self.entries.next();
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        if let Some(entry) = self.entries.next() {
            seed.deserialize(ValueDeserializer::new(entry.value()))
        } else {
            Err(Error::Message("Expected value".into()))
        }
    }
}

/// Deserializer for a single KDL value.
struct ValueDeserializer<'de> {
    value: &'de KdlValue,
}

impl<'de> ValueDeserializer<'de> {
    fn new(value: &'de KdlValue) -> Self {
        Self { value }
    }
}

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            KdlValue::String(s) => visitor.visit_str(s),
            KdlValue::Integer(n) => visitor.visit_i64(*n as i64),
            KdlValue::Float(f) => visitor.visit_f64(*f),
            KdlValue::Bool(b) => visitor.visit_bool(*b),
            KdlValue::Null => visitor.visit_none(),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(b) = self.value.as_bool() {
            visitor.visit_bool(b)
        } else {
            Err(Error::TypeMismatch {
                expected: "bool".into(),
                got: format!("{:?}", self.value),
            })
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(n) = self.value.as_integer() {
            visitor.visit_i64(n as i64)
        } else {
            Err(Error::TypeMismatch {
                expected: "integer".into(),
                got: format!("{:?}", self.value),
            })
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(n) = self.value.as_integer() {
            visitor.visit_u64(n as u64)
        } else {
            Err(Error::TypeMismatch {
                expected: "unsigned integer".into(),
                got: format!("{:?}", self.value),
            })
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(n) = self.value.as_float() {
            visitor.visit_f64(n)
        } else {
            Err(Error::TypeMismatch {
                expected: "float".into(),
                got: format!("{:?}", self.value),
            })
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(s) = self.value.as_string() {
            visitor.visit_str(s)
        } else {
            Err(Error::TypeMismatch {
                expected: "string".into(),
                got: format!("{:?}", self.value),
            })
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.value.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// Simple string deserializer.
struct StrDeserializer<'de>(&'de str);

impl<'de> de::Deserializer<'de> for StrDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

/// Sequence deserializer.
struct SeqDeserializer<I> {
    iter: I,
}

impl<I> SeqDeserializer<I> {
    fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'de, I> SeqAccess<'de> for SeqDeserializer<I>
where
    I: Iterator<Item = &'de KdlNode>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(node) => seed.deserialize(NodeDeserializer::new(node)).map(Some),
            None => Ok(None),
        }
    }
}

/// Enum deserializer.
struct EnumDeserializer<'de> {
    node: &'de KdlNode,
}

impl<'de> EnumDeserializer<'de> {
    fn new(node: &'de KdlNode) -> Self {
        Self { node }
    }
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = Error;
    type Variant = VariantDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StrDeserializer(self.node.name().value()))?;
        Ok((variant, VariantDeserializer::new(self.node)))
    }
}

/// Variant deserializer.
struct VariantDeserializer<'de> {
    node: &'de KdlNode,
}

impl<'de> VariantDeserializer<'de> {
    fn new(node: &'de KdlNode) -> Self {
        Self { node }
    }
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(NodeDeserializer::new(self.node))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(NodeDeserializer::new(self.node), visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(NodeDeserializer::new(self.node), visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_deserialize_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Config {
            name: String,
            port: u16,
            debug: bool,
        }

        let kdl = r#"config name="my-app" port=8080 debug=#true"#;
        let config: Config = from_str(kdl).unwrap();

        assert_eq!(config.name, "my-app");
        assert_eq!(config.port, 8080);
        assert!(config.debug);
    }

    #[test]
    fn test_deserialize_optional() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Config {
            name: String,
            #[serde(default)]
            port: Option<u16>,
        }

        let kdl = r#"config name="my-app""#;
        let config: Config = from_str(kdl).unwrap();

        assert_eq!(config.name, "my-app");
        assert_eq!(config.port, None);
    }
}
