//! KDL Serializer implementation.

use kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue};
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, SerializeStruct};

use crate::Error;

/// Serialize a type to a KDL string.
pub fn to_string<T>(value: &T) -> Result<String, Error>
where
    T: Serialize + ?Sized,
{
    let node = to_node("root", value)?;
    Ok(node.to_string())
}

/// Serialize a type to a KDL string with a custom node name.
pub fn to_string_with_name<T>(name: &str, value: &T) -> Result<String, Error>
where
    T: Serialize + ?Sized,
{
    let node = to_node(name, value)?;
    Ok(node.to_string())
}

/// Serialize a type to a KdlNode.
pub fn to_node<T>(name: &str, value: &T) -> Result<KdlNode, Error>
where
    T: Serialize + ?Sized,
{
    let serializer = NodeSerializer::new(name);
    value.serialize(serializer)
}

/// Serializer that produces a KdlNode.
struct NodeSerializer {
    name: String,
}

impl NodeSerializer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl ser::Serializer for NodeSerializer {
    type Ok = KdlNode;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = SeqSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::Bool(v)));
        Ok(node)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::Integer(v as i128)));
        Ok(node)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::Integer(v as i128)));
        Ok(node)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::Float(v)));
        Ok(node)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::String(v.to_string())));
        Ok(node)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        node.push(KdlEntry::new(KdlValue::Null));
        Ok(node)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(KdlNode::new(self.name))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(KdlNode::new(variant))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(NodeSerializer::new(variant))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(self.name))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SeqSerializer::new(variant.to_string()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(self.name))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer::new(self.name))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructSerializer::new(variant.to_string()))
    }
}

/// Serializer for sequences.
struct SeqSerializer {
    name: String,
    children: Vec<KdlNode>,
}

impl SeqSerializer {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
        }
    }
}

impl SerializeSeq for SeqSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let node = value.serialize(NodeSerializer::new("-"))?;
        self.children.push(node);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut node = KdlNode::new(self.name);
        if !self.children.is_empty() {
            let mut doc = KdlDocument::new();
            for child in self.children {
                doc.nodes_mut().push(child);
            }
            node.set_children(doc);
        }
        Ok(node)
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for SeqSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

/// Serializer for maps.
struct MapSerializer {
    node: KdlNode,
    current_key: Option<String>,
}

impl MapSerializer {
    fn new(name: impl Into<String>) -> Self {
        Self {
            node: KdlNode::new(name.into()),
            current_key: None,
        }
    }
}

impl SerializeMap for MapSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let key_str = key.serialize(StringSerializer)?;
        self.current_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let key = self.current_key.take().ok_or_else(|| {
            Error::Message("serialize_value called before serialize_key".into())
        })?;

        let kdl_value = value.serialize(ValueSerializer)?;
        let mut entry = KdlEntry::new(kdl_value);
        entry.set_name(Some(KdlIdentifier::from(key)));
        self.node.push(entry);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.node)
    }
}

/// Serializer for structs.
struct StructSerializer {
    node: KdlNode,
}

impl StructSerializer {
    fn new(name: impl Into<String>) -> Self {
        Self {
            node: KdlNode::new(name.into()),
        }
    }
}

impl SerializeStruct for StructSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let kdl_value = value.serialize(ValueSerializer)?;
        let mut entry = KdlEntry::new(kdl_value);
        entry.set_name(Some(KdlIdentifier::from(key)));
        self.node.push(entry);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.node)
    }
}

impl ser::SerializeStructVariant for StructSerializer {
    type Ok = KdlNode;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeStruct::end(self)
    }
}

/// Serializer that produces a KdlValue.
struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = KdlValue;
    type Error = Error;

    type SerializeSeq = ser::Impossible<KdlValue, Error>;
    type SerializeTuple = ser::Impossible<KdlValue, Error>;
    type SerializeTupleStruct = ser::Impossible<KdlValue, Error>;
    type SerializeTupleVariant = ser::Impossible<KdlValue, Error>;
    type SerializeMap = ser::Impossible<KdlValue, Error>;
    type SerializeStruct = ser::Impossible<KdlValue, Error>;
    type SerializeStructVariant = ser::Impossible<KdlValue, Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Integer(v as i128))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Integer(v as i128))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::String(v.to_string()))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(KdlValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(Error::UnsupportedType)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }
}

/// Serializer that produces a String.
struct StringSerializer;

impl ser::Serializer for StringSerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.to_string())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(Error::UnsupportedType)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_serialize_struct() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            port: u16,
            debug: bool,
        }

        let config = Config {
            name: "my-app".into(),
            port: 8080,
            debug: true,
        };

        let kdl = to_string(&config).unwrap();
        // KDL output format: root name="my-app" port=8080 debug=#true
        assert!(kdl.contains("my-app"), "KDL output: {}", kdl);
        assert!(kdl.contains("8080"), "KDL output: {}", kdl);
        assert!(kdl.contains("#true"), "KDL output: {}", kdl);
    }
}
