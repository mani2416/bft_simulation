//! The conv module provides serveral methods for type conversion mostly vec<u8> from/to others

extern crate base64;
extern crate encoding;
use serde::{Deserialize, Deserializer, Serializer};

/// Serializes `buffer` to a base64-string. Only usefull with serde.
/// can be applied to structs by:
/// #[serde(serialize_with = "vec_u8_to_str", deserialize_with = "str_to_vec_u8")]
#[deprecated(
    since = "0.4.9",
    note = "use mc_utils::conv::ser_vec_u8_to_str instead for clearer naming"
)]
pub fn vec_u8_to_str<T, S>(buffer: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    ser_vec_u8_to_str(buffer, serializer)
}
/// Serializes `buffer` to a base64-string. Only usefull with serde.
/// can be applied to structs by:
/// #[serde(serialize_with = "vec_u8_to_str", deserialize_with = "str_to_vec_u8")]
pub fn ser_vec_u8_to_str<T, S>(buffer: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    serializer.serialize_str(&base64::encode_config(buffer.as_ref(), base64::STANDARD))
}

/// Deserializes a string formatted in base64 to a `Vec<u8>`. Only usefull with serde.
/// can be applied to structs by:
/// #[serde(serialize_with = "vec_u8_to_str", deserialize_with = "str_to_vec_u8")]
#[deprecated(
    since = "0.4.9",
    note = "use mc_utils::conv::de_str_to_vec_u8 instead for clearer naming"
)]
pub fn str_to_vec_u8<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    de_str_to_vec_u8(deserializer)
}
/// Deserializes a string formatted in base64 to a `Vec<u8>`. Only usefull with serde.
/// can be applied to structs by:
/// #[serde(serialize_with = "vec_u8_to_str", deserialize_with = "str_to_vec_u8")]
pub fn de_str_to_vec_u8<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        // .and_then(|string| hex::decode(string).map_err(|err| Error::custom(err.to_string())))
        .and_then(|string| {
            base64::decode_config(string.as_bytes(), base64::STANDARD)
                .map_err(|err| Error::custom(err.to_string()))
        })
}

use encoding::all::UTF_8;
use encoding::{DecoderTrap, Encoding};
/// print a Vec<u8> as String encoded in UTF8 and escaping "unprintable" Bytes
pub fn vec_u8_to_string(bytes: &[u8]) -> String {
    UTF_8
        .decode(bytes, DecoderTrap::Replace)
        .expect("cannot escape invalid Byte")
}

/// converting u32 to vec<u8>
pub fn u32_to_vec(int: u32) -> Vec<u8> {
    let mut result = Vec::new();
    result.push((int % 256) as u8);
    let mut int = int / 256;
    result.push((int % 256) as u8);
    int /= 256;
    result.push((int % 256) as u8);
    int /= 256;
    result.push(int as u8);
    result
}
