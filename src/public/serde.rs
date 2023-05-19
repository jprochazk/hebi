use serde::de::DeserializeSeed;

use crate::{Bind, Global, Value};

pub struct ValueDeserializer<'cx> {
  global: Global<'cx>,
}

impl<'cx> ValueDeserializer<'cx> {
  pub fn new(global: Global<'cx>) -> Self {
    Self { global }
  }
}

impl<'de, 'cx> DeserializeSeed<'de> for ValueDeserializer<'cx> {
  type Value = Value<'cx>;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    crate::serde::ValueDeserializer {
      global: self.global.inner,
    }
    .deserialize(deserializer)
    .map(|value| unsafe { value.bind_raw::<'cx>() })
  }
}
