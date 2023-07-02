use std::string::String as StdString;

use serde::de::{DeserializeSeed, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq};

use super::object::{List, Ptr, Str, Table};
use super::value::Value;
use super::vm::global::Global;
use crate::util::{MAX_SAFE_INT, MIN_SAFE_INT};

impl Serialize for Value {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    if self.is_float() {
      let value = unsafe { self.clone().to_float_unchecked() };
      serializer.serialize_f64(value)
    } else if self.is_int() {
      let value = unsafe { self.clone().to_int_unchecked() };
      serializer.serialize_i32(value)
    } else if self.is_bool() {
      let value = unsafe { self.clone().to_bool_unchecked() };
      serializer.serialize_bool(value)
    } else if self.is_none() {
      serializer.serialize_none()
    } else if self.is_object() {
      let value = unsafe { self.clone().to_any_unchecked() };

      if value.is::<Str>() {
        let value = unsafe { value.cast_unchecked::<Str>() };
        value.serialize(serializer)
      } else if value.is::<Table>() {
        let value = unsafe { value.cast_unchecked::<Table>() };
        value.serialize(serializer)
      } else if value.is::<List>() {
        let value = unsafe { value.cast_unchecked::<List>() };
        value.serialize(serializer)
      } else {
        Err(serde::ser::Error::custom(format!(
          "cannot serialize `{value}`"
        )))
      }
    } else {
      Err(serde::ser::Error::custom(format!(
        "cannot serialize `{self}`"
      )))
    }
  }
}

impl Serialize for Table {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut map = serializer.serialize_map(Some(self.len()))?;
    for (key, value) in self.entries() {
      map.serialize_entry(key.as_ref(), &value)?;
    }
    map.end()
  }
}

impl Serialize for List {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut list = serializer.serialize_seq(Some(self.len()))?;
    for value in self.iter() {
      list.serialize_element(&value)?;
    }
    list.end()
  }
}

impl Serialize for Str {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.as_str())
  }
}

macro_rules! try_to_f64 {
  ($ty:ty, $v:expr) => {{
    let v = $v;
    let fv = v as f64;
    if fv < MIN_SAFE_INT {
      Err(serde::de::Error::custom(format!(
        "{} is out of bounds ({} < {})",
        std::any::type_name::<$ty>(),
        v,
        MIN_SAFE_INT
      )))
    } else if fv > MAX_SAFE_INT {
      Err(serde::de::Error::custom(format!(
        "{} is out of bounds ({} < {})",
        std::any::type_name::<$ty>(),
        v,
        MAX_SAFE_INT
      )))
    } else {
      Ok(fv)
    }
  }};
}

pub struct ValueDeserializer {
  pub global: Global,
}

impl<'de> DeserializeSeed<'de> for ValueDeserializer {
  type Value = Value;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_any(ValueVisitor {
      global: self.global,
    })
  }
}

struct StringDeserializer {
  global: Global,
}

impl<'de> DeserializeSeed<'de> for StringDeserializer {
  type Value = Ptr<Str>;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    struct V {
      global: Global,
    }

    impl<'de> Visitor<'de> for V {
      type Value = Ptr<Str>;

      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
      }

      fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok(self.global.alloc(Str::owned(v)))
      }

      fn visit_string<E>(self, v: StdString) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok(self.global.alloc(Str::owned(v)))
      }

      fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
      where
        E: serde::de::Error,
      {
        Ok(self.global.alloc(Str::owned(v)))
      }
    }

    deserializer.deserialize_str(V {
      global: self.global,
    })
  }
}

struct ValueVisitor {
  global: Global,
}

impl<'de> Visitor<'de> for ValueVisitor {
  type Value = Value;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("a value")
  }

  fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::bool(v))
  }

  fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_i32(v as i32)
  }

  fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_i32(v as i32)
  }

  fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::int(v))
  }

  fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
      try_to_f64!(i64, v).map(Value::float)
    } else {
      Ok(Value::int(v as i32))
    }
  }

  fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    if v < i32::MIN as i128 || v > i32::MAX as i128 {
      try_to_f64!(i128, v).map(Value::float)
    } else {
      Ok(Value::int(v as i32))
    }
  }

  fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_u32(v as u32)
  }

  fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_u32(v as u32)
  }

  fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    if v > i32::MAX as u32 {
      try_to_f64!(u32, v).map(Value::float)
    } else {
      Ok(Value::int(v as i32))
    }
  }

  fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    if v > i32::MAX as u64 {
      try_to_f64!(u64, v).map(Value::float)
    } else {
      Ok(Value::int(v as i32))
    }
  }

  fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    if v > i32::MAX as u128 {
      try_to_f64!(u128, v).map(Value::float)
    } else {
      Ok(Value::int(v as i32))
    }
  }

  fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_f64(v as f64)
  }

  fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::float(v))
  }

  fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::object(self.global.alloc(Str::owned(v))))
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::object(self.global.alloc(Str::owned(v))))
  }

  fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::object(self.global.alloc(Str::owned(v))))
  }

  fn visit_string<E>(self, v: StdString) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::object(self.global.alloc(Str::owned(v))))
  }

  // TODO: some kind of Bytes object?
  /*
  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    let _ = v;
    Err(serde::de::Error::invalid_type(
      serde::de::Unexpected::Bytes(v),
      &self,
    ))
  }
  */
  /*
  fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_bytes(v)
  }
  */
  /*
  fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    self.visit_bytes(&v)
  }
  */

  fn visit_none<E>(self) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::none())
  }

  fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    ValueDeserializer {
      global: self.global,
    }
    .deserialize(deserializer)
  }

  fn visit_unit<E>(self) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(Value::none())
  }

  // TODO: no clue what this is
  /*
  fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let _ = deserializer;
    Err(serde::de::Error::invalid_type(
      serde::de::Unexpected::NewtypeStruct,
      &self,
    ))
  }
  */

  fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
  where
    A: serde::de::SeqAccess<'de>,
  {
    let list = self
      .global
      .alloc(List::with_capacity(seq.size_hint().unwrap_or(0)));
    while let Some(value) = seq.next_element_seed(ValueDeserializer {
      global: self.global.clone(),
    })? {
      list.push(value);
    }
    Ok(Value::object(list))
  }

  fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
  where
    A: serde::de::MapAccess<'de>,
  {
    let table = self
      .global
      .alloc(Table::with_capacity(map.size_hint().unwrap_or(0)));
    while let Some((key, value)) = map.next_entry_seed(
      StringDeserializer {
        global: self.global.clone(),
      },
      ValueDeserializer {
        global: self.global.clone(),
      },
    )? {
      table.insert(key, value);
    }
    Ok(Value::object(table))
  }

  // TODO: i dont think i can represent this in Hebi...
  /*
  fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
  where
    A: serde::de::EnumAccess<'de>,
  {
    let _ = data;
    Err(serde::de::Error::invalid_type(
      serde::de::Unexpected::Enum,
      &self,
    ))
  }
  */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deserialize_int() {
    let global = Global::default();

    let json = r#"5360574452"#;

    let value = ValueDeserializer { global }
      .deserialize(&mut serde_json::Deserializer::from_str(json))
      .unwrap();

    assert_eq!(value.to_float(), Some(5360574452_f64));
  }
}
