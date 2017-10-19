//! A UUID wrapper that has a base64 display and serialization
//!
//! # What?
//!
//! A newtype around UUIDs that:
//!
//! * Displays and Serializes as Base64
//!   * Specifically it is the url-safe base64 variant, *with no padding*
//!
//! ```rust
//! # extern crate uuid;
//! # extern crate uuid_b64;
//! # use uuid::Uuid;
//! # use uuid_b64::UuidB64;
//! # fn main() {
//! let known_id = Uuid::parse_str("b0c1ee86-6f46-4f1b-8d8b-7849e75dbcee").unwrap();
//! let as_b64 = UuidB64::from(known_id);
//! assert_eq!(as_b64.to_string(), "sMHuhm9GTxuNi3hJ51287g");
//!
//! let parsed_b64: UuidB64 = "sMHuhm9GTxuNi3hJ51287g".parse().unwrap();
//! assert_eq!(parsed_b64, as_b64);
//!
//! let raw_id = Uuid::new_v4();
//! assert_eq!(raw_id.to_string().len(), 36);
//! let uuidb64 = UuidB64::from(raw_id);
//! assert_eq!(uuidb64.to_string().len(), 22);
//! # }
//! ```
//!
//! `UuidB64::new` creates v4 UUIDs, because... that's what I use. I'm open to
//! hearing arguments about why this is a ridiculous decision and I should have
//! made `new` be `new_v4`.
//!
//! # Why?
//!
//! UUIDs are great:
//!
//! * They have a known size and representation, meaning that they are
//!   well-supported with an efficient representation in a wide variety of
//!   systems. Things like programming languages and databases.
//! * V4 (almost completely random) UUIDs have nice sharding properties, you
//!   can give out UUIDs willy-nilly without coordination and still be
//!   guaranteed to not have a conflict of IDs between two items across
//!   systems.
//!
//! That said, the standard *representation* for UUIDs is kind of annoying:
//!
//! * It's a *long*: 36 characters to represent 16 bytes of data!
//! * It's hard to read: it is only hexadecimal characters. The human eye needs
//!   to pay a lot of attention to be certain if any two UUIDs are the same.
//!
//! I guess that's it. Base64 is a more human-friendly representation of UUIDs:
//!
//! * It's slightly shorter: 1.375 times the size of the raw data (22
//!   characters), vs 2.25 times the size characters.
//! * Since it is case-sensitive, the *shape* of the IDs helps to distinguish
//!   between different IDs. There is also more entropy per character, so
//!   scanning to find a difference is faster.
//!
//! That said, there are drawbacks to something like this:
//!
//! * If you store it as a UUID column in a database IDs won't show up in the
//!   same way as it does in your application code, meaning you'll (A) maybe
//!   want to define a new database type, or B just expect to only ever
//!   interact with the DB via you application.
//!
//!   Conversion functions are pretty trivial, this works in postgres
//!   (inefficiently, but it's only for interactive queries so whatever):
//!
//!   ```sql
//!   CREATE FUNCTION b64uuid(encoded TEXT) RETURNS UUID
//!   AS $$
//!       BEGIN
//!           RETURN ENCODE(DECODE(REPLACE(REPLACE(
//!               encoded, '-', '+'), '_', '/') || '==', 'base64'), 'hex')::UUID;
//!       END
//!   $$ LANGUAGE plpgsql;
//!   ```
//!
//! # Usage
//!
//! Just use `UuidB64` everywhere you would use `Uuid`, and use `UuidB64::from`
//! to create one from an existing UUID.
//!
//! ## Features
//!
//! * `serde` enables serialization/deserialization via Serde.

extern crate base64;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate uuid;

#[cfg(all(test, feature = "serde"))]
#[macro_use]
extern crate serde_derive;
#[cfg(all(test, feature = "serde"))]
#[macro_use]
extern crate serde_json;

use std::convert::From;
use std::str::FromStr;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use uuid::Uuid;
use base64::{CharacterSet, Config, LineWrap};
use base64::display::Base64Display;

use errors::{ErrorKind, ResultExt};

mod errors;
#[cfg(feature = "serde")]
mod serde_impl;

lazy_static! {
    static ref B64_CONFIG: Config = Config::new(
        CharacterSet::UrlSafe,
        false,
        true,
        LineWrap::NoWrap,
    );
}

/// It's a Uuid that displays as Base 64
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct UuidB64(uuid::Uuid);

impl UuidB64 {
    /// Generate a new v4 Uuid
    pub fn new() -> UuidB64 {
        UuidB64(Uuid::new_v4())
    }

    /// Get the raw UUID out
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl FromStr for UuidB64 {
    type Err = errors::ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes =
            base64::decode_config(s, *B64_CONFIG).chain_err(|| ErrorKind::ParseError(s.into()))?;
        let id = Uuid::from_bytes(&bytes).chain_err(|| ErrorKind::ParseError(s.into()))?;
        Ok(UuidB64(id))
    }
}

/// Right now this is just Uuid, but anything Uuid is comfortable with, we are
impl<T> From<T> for UuidB64
where
    T: Into<Uuid>,
{
    fn from(item: T) -> Self {
        UuidB64(item.into())
    }
}

impl Debug for UuidB64 {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "UuidB64({})", self)
    }
}

impl Display for UuidB64 {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        // can only hit this error if we use an invalid line length
        let wrapper = Base64Display::with_config(self.0.as_bytes(), *B64_CONFIG).unwrap();
        write!(f, "{}", wrapper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_b64() {
        let id = UuidB64::new();
        let fmted = format!("{}", id);
        assert_eq!(fmted.len(), 22);
        assert_eq!(format!("UuidB64({})", fmted), format!("{:?}", id));
    }

    #[test]
    fn parse_roundtrips() {
        let original = UuidB64::new();
        let encoded = format!("{}", original);
        let parsed: UuidB64 = encoded.parse().unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_uuid_works() {
        let _ = UuidB64::from(Uuid::new_v4());
    }
}
