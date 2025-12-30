use crate::{Error, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct Call(u8);

impl Call {
    pub fn new(call: u8) -> Result<Self> {
        match call {
            v if v < 1 => Err(Error::CallValueTooSmall),
            v if v > 13 => Err(Error::CallValueTooLarge),
            _ => Ok(Call(call)),
        }
    }
}

impl<'de> Deserialize<'de> for Call {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let call = u8::deserialize(deserializer)?;
        match Call::new(call) {
            Ok(_) => Ok(Call(call)),
            Err(e) => Err(serde::de::Error::custom(e.to_string())),
        }
    }
}
