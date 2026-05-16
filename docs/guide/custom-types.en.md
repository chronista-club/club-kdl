# Custom Types Guide

English | **[日本語](./custom-types.md)**

How to map types that club-kdl does not support out of the box (external crate
types, your own newtypes, etc.) to KDL values.

## Overview

club-kdl supports the following types out of the box:

- Integers (`i32` `i64` `i128` `u16` `u32` `u64` `usize`) / floats (`f64`) / booleans (`bool`)
- Strings (`String` `&str`) / paths (`PathBuf`)
- Collections (`Vec<T>` `HashMap<String, String>`) / `Option<T>`

To use any other type ―― for example `chrono::NaiveDate`, your own `UserId(u64)`,
or an enum from an external crate ―― in a `#[kdl(argument)]` / `#[kdl(property)]`
field, **implement the `FromKdlValue` / `ToKdlValue` traits** for that type.

## Prerequisites

- Familiarity with club-kdl basics (`#[derive(KdlDeserialize, KdlSerialize)]` and `#[kdl(...)]` attributes)
- The target is a type used as a "scalar value" mapped to an argument / property. Types with child-node structure are expressed as ordinary structs with derive.

## The traits

```rust
use club_kdl::{FromKdlValue, ToKdlValue};

pub trait FromKdlValue<'de>: Sized {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self>;
}

pub trait ToKdlValue {
    fn to_kdl_value(&self) -> KdlValue;
}
```

- `FromKdlValue` is needed for deserialization, `ToKdlValue` for serialization. Implementing only one is fine.
- The `'de` lifetime enables zero-copy borrowing from the KDL source. Call `.to_owned()` if you need to own the value.

## Usage

### Example 1: newtype ―― a thin wrapper over an existing type

A newtype like `UserId(u64)` simply delegates to the inner type's implementation.

```rust
use club_kdl::{FromKdlValue, ToKdlValue, KdlValue, Result};

struct UserId(u64);

impl<'de> FromKdlValue<'de> for UserId {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        u64::from_kdl_value(value).map(UserId)
    }
}

impl ToKdlValue for UserId {
    fn to_kdl_value(&self) -> KdlValue {
        self.0.to_kdl_value()
    }
}
```

### Example 2: external type ―― mapping `chrono::NaiveDate` to a string

You **cannot** implement club-kdl's traits directly on an external crate's type
(`chrono::NaiveDate`) because of the **orphan rule** (both the type and the trait
live outside your crate). Work around it by **wrapping in a newtype**.

```rust
use club_kdl::{FromKdlValue, ToKdlValue, KdlValue, Error, Result};
use chrono::NaiveDate;

struct Date(NaiveDate);

impl<'de> FromKdlValue<'de> for Date {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        // First extract as &str, then parse with chrono
        let s = <&str>::from_kdl_value(value)?;
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| Error::Custom(e.to_string()))
    }
}

impl ToKdlValue for Date {
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.0.format("%Y-%m-%d").to_string())
    }
}
```

Now you can use it as `#[kdl(property)] created: Date`:

```kdl
record created="2026-05-17"
```

### Example 3: validating the value

`from_kdl_value` returns a `Result`, so you can validate while parsing.

```rust
struct Port(u16);

impl<'de> FromKdlValue<'de> for Port {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        let n = u16::from_kdl_value(value)?;
        if n == 0 {
            return Err(Error::Custom("port must not be 0".into()));
        }
        Ok(Port(n))
    }
}
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `only traits defined in the current crate can be implemented for types defined outside of the crate` | orphan rule ―― tried to implement an external trait on an external type | Wrap in a newtype (Example 2) |
| `borrowed value does not live long enough` | tried to convert while borrowing `&'de str` | If ownership is needed, go through `String` with `.to_owned()` |
| Want to return a type mismatch | The KDL value kind differs from what you expect | Return `Error::type_mismatch("expected", value)` |

## See also

- The "Supported types" section of the README
- Scalar enum mapping is handled automatically by derive (`#[kdl(rename = "...")]`) ―― no manual implementation needed
