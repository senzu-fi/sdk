//! `u64` / `u128` as strings: JavaScript numbers lose precision above 2^53. Serializes decimal, reads decimal or
//! `0x` hex.

extern crate alloc;

use core::fmt::Display;

use serde::{Deserialize, Deserializer, Serializer, de::Error};

pub(crate) trait WireInt: Sized + Display {
    fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError>;
}

impl WireInt for u64 {
    fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError> {
        Self::from_str_radix(src, radix)
    }
}

impl WireInt for u128 {
    fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError> {
        Self::from_str_radix(src, radix)
    }
}

pub(crate) fn serialize<T: WireInt, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

pub(crate) fn deserialize<'de, T: WireInt, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
    let text = <alloc::borrow::Cow<'de, str>>::deserialize(deserializer)?;
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => T::from_str_radix(hex, 16),
        None => T::from_str_radix(&text, 10),
    };
    parsed.map_err(D::Error::custom)
}
