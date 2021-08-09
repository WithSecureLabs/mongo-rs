//! Extensions for the `bson` crate so that `mongo-derive` can be implemented efficiently.

use serde::de::Error;
use serde::ser::Error as SerError;
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::iter::FromIterator;

// The BSON crate implments zero methods to go from BSON to another type without using serde, lets
// rectify that here...
// NOTE: We can add more types in here as required...
// NOTE: We also do a bit of type massaging which means we are not gonna be as space efficient as
// possible... and that we could encounter type confusion!

pub mod de {
    //! Extends `bson`'s deserialisation error with `Infallible` so that is can be used in rust
    //! conversions.
    pub use serde::de::Error as ErrorExt;
    use std::convert::Infallible;
    use std::error::Error as StdError;
    use std::fmt;

    /// An error that extends `bson`'s deserialisation error.
    pub struct Error(pub bson::de::Error);

    impl fmt::Debug for Error {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(fmt)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl StdError for Error {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.0.source()
        }
    }

    impl From<bson::de::Error> for Error {
        fn from(err: bson::de::Error) -> Self {
            Error(err)
        }
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!()
        }
    }
}

pub mod ser {
    //! Extends `bson`'s serialisation error with `Infallible` so that is can be used in rust
    //! conversions.
    use std::convert::Infallible;
    use std::error::Error as StdError;
    use std::fmt;

    /// An error that extends `bson`'s serialisation error.
    pub struct Error(pub bson::ser::Error);

    impl fmt::Debug for Error {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(fmt)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl StdError for Error {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.0.source()
        }
    }

    impl From<bson::ser::Error> for Error {
        fn from(err: bson::ser::Error) -> Self {
            Error(err)
        }
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!()
        }
    }
}

/// Wraps `bson::Bson` so that additional rust conversions can be applied.
pub struct Bson(pub bson::Bson);

/// Wraps a type that implements `serde::de::Deserialize` so it can bypass blanket implementations
// FIXME: https://github.com/rust-lang/rust/issues/31844
pub struct De<T: serde::de::DeserializeOwned>(pub T);

/// Wraps a type that implements `serde::ser::Serialize` so it can bypass blanket implementations
// FIXME: https://github.com/rust-lang/rust/issues/31844
pub struct Ser<T: serde::ser::Serialize>(pub T);

// NOTE: Due to https://github.com/rust-lang/rust/issues/29635 we cant be generic and implement the
// missing, so we have to wrap them all... yay...
macro_rules! wrap_bson_from {
    ($source:ty) => {
        impl From<$source> for Bson {
            fn from(a: $source) -> Self {
                Bson(a.into())
            }
        }
    };
}
wrap_bson_from!(bson::Binary);
wrap_bson_from!(bson::Bson);
wrap_bson_from!(bson::DbPointer);
wrap_bson_from!(bson::Document);
wrap_bson_from!(bson::JavaScriptCodeWithScope);
wrap_bson_from!(bson::oid::ObjectId);
wrap_bson_from!(bson::Regex);
wrap_bson_from!(bson::Timestamp);
wrap_bson_from!(&str);
wrap_bson_from!(bool);
wrap_bson_from!(f32);
wrap_bson_from!(f64);
wrap_bson_from!(i32);
wrap_bson_from!(i64);
wrap_bson_from!(String);
wrap_bson_from!([u8; 12]);
#[cfg(feature = "chrono")]
wrap_bson_from!(chrono::DateTime<chrono::Utc>);

impl From<char> for Bson {
    fn from(c: char) -> Self {
        Bson(bson::Bson::String(c.into()))
    }
}

impl From<i8> for Bson {
    fn from(value: i8) -> Self {
        Bson(bson::Bson::Int32(value as i32))
    }
}

impl From<i16> for Bson {
    fn from(value: i16) -> Self {
        Bson(bson::Bson::Int32(value as i32))
    }
}

impl From<u8> for Bson {
    fn from(value: u8) -> Self {
        Bson(bson::Bson::Int32(value as i32))
    }
}

impl From<u16> for Bson {
    fn from(value: u16) -> Self {
        Bson(bson::Bson::Int32(value as i32))
    }
}

impl From<u32> for Bson {
    fn from(value: u32) -> Self {
        Bson(bson::Bson::Int32(value as i32))
    }
}

impl From<u64> for Bson {
    fn from(value: u64) -> Self {
        Bson(bson::Bson::Int64(value as i64))
    }
}

impl<T> TryFrom<Bson> for De<T>
where
    T: serde::de::DeserializeOwned,
{
    type Error = de::Error;
    fn try_from(value: Bson) -> Result<Self, Self::Error> {
        Ok(De(
            bson::from_bson(value.0).map_err(|e| bson::de::Error::custom(e))?
        ))
    }
}

impl<T> TryFrom<Ser<T>> for Bson
where
    T: serde::ser::Serialize,
{
    type Error = ser::Error;
    fn try_from(value: Ser<T>) -> Result<Self, Self::Error> {
        Ok(Bson(
            bson::to_bson(&value.0).map_err(|e| bson::ser::Error::custom(e))?,
        ))
    }
}

impl<K, V> TryFrom<HashMap<K, V>> for Bson
where
    K: Into<String>,
    V: TryInto<Bson>,
    V::Error: Into<ser::Error>,
{
    type Error = ser::Error;
    fn try_from(m: HashMap<K, V>) -> Result<Self, Self::Error> {
        let mut doc = bson::Document::new();
        for (k, v) in m {
            doc.insert(k.into(), v.try_into().map_err(|e| e.into())?.0);
        }
        Ok(Bson(bson::Bson::Document(doc)))
    }
}

impl<T> TryFrom<HashSet<T>> for Bson
where
    T: Eq + Hash + TryInto<Bson>,
    T::Error: Into<ser::Error>,
{
    type Error = ser::Error;
    fn try_from(s: HashSet<T>) -> Result<Self, Self::Error> {
        let int = s
            .into_iter()
            .map(|t| t.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())?;
        Ok(Bson(bson::Bson::Array(
            int.into_iter().map(|x| x.0).collect(),
        )))
    }
}

impl<T> TryFrom<Option<T>> for Bson
where
    T: TryInto<Bson>,
    T::Error: Into<ser::Error>,
{
    type Error = ser::Error;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        Ok(match value {
            Some(v) => v.try_into().map_err(|e| e.into())?,
            None => Bson(bson::Bson::Null),
        })
    }
}

impl<T> From<Vec<T>> for Bson
where
    T: Into<Bson>,
{
    fn from(v: Vec<T>) -> Self {
        Bson(bson::Bson::Array(
            v.into_iter().map(|x| x.into().0).collect(),
        ))
    }
}

impl<T> From<&T> for Bson
where
    T: Clone + Into<Bson>,
{
    fn from(t: &T) -> Self {
        t.clone().into()
    }
}

impl<T> From<&[T]> for Bson
where
    T: Clone + Into<Bson>,
{
    fn from(v: &[T]) -> Self {
        Bson(bson::Bson::Array(
            v.iter().cloned().map(|x| x.into().0).collect(),
        ))
    }
}

impl<T> FromIterator<T> for Bson
where
    T: Into<Bson>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Bson(bson::Bson::Array(
            iter.into_iter().map(Into::into).map(|x| x.0).collect(),
        ))
    }
}

impl TryFrom<Bson> for Box<bson::Bson> {
    type Error = std::convert::Infallible;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        Ok(Box::new(bson.0))
    }
}

impl From<Bson> for Option<bson::Bson> {
    fn from(bson: Bson) -> Self {
        let inner = bson.0;
        match inner {
            bson::Bson::Null => None,
            _ => Some(inner),
        }
    }
}

impl TryFrom<Bson> for bool {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Boolean(b) => Ok(b),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Bool(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for char {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::String(s) => {
                if s.len() == 1 {
                    return Ok(s.chars().next().unwrap());
                }
                Err(bson::de::Error::custom(format!(
                    "invalid value, expected a char but found a string `{}`",
                    s
                )))
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::String(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for f32 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Double(f) => {
                if f < (Self::MIN as f64) || f > (Self::MAX as f64) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an f32",
                        f
                    )));
                }
                Ok(f as f32)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Double(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for f64 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Double(f) => Ok(f),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Double(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for i8 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => {
                if i < (Self::MIN as i32) || i > (Self::MAX as i32) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an i8",
                        i
                    )));
                }
                Ok(i as i8)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for i16 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => {
                if i < (Self::MIN as i32) || i > (Self::MAX as i32) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an i16",
                        i
                    )));
                }
                Ok(i as i16)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for i32 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => Ok(i),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for i64 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int64(i) => Ok(i),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int64(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for isize {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int64(i) => {
                if i < (Self::MIN as i64) || i > (Self::MAX as i64) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an isize",
                        i
                    )));
                }
                Ok(i as isize)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int64(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for u8 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => {
                if i < (Self::MIN as i32) || i > (Self::MAX as i32) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an u8",
                        i
                    )));
                }
                Ok(i as u8)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for u16 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => {
                if i < (Self::MIN as i32) || i > (Self::MAX as i32) {
                    return Err(bson::de::Error::custom(format!(
                        "invalid value, could not coerce `{}` into an u16",
                        i
                    )));
                }
                Ok(i as u16)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for u32 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int32(i) => Ok(i as u32),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int32(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for u64 {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int64(i) => Ok(i as u64),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int64(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for usize {
    type Error = bson::de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Int64(i) => Ok(i as usize),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Int64(...)` but found `{}`",
                inner
            ))),
        }
    }
}

impl TryFrom<Bson> for String {
    type Error = de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::String(s) => Ok(s),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::String(...)` but found `{}`",
                inner
            ))
            .into()),
        }
    }
}

impl<K, V> TryFrom<Bson> for HashMap<K, V>
where
    K: Eq + Hash + TryFrom<String>,
    K::Error: Into<de::Error>,
    V: TryFrom<Bson>,
    V::Error: Into<de::Error>,
{
    type Error = de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Document(d) => {
                let mut m = HashMap::with_capacity(d.len());
                for (k, v) in d {
                    m.insert(
                        K::try_from(k).map_err(|e| e.into())?,
                        V::try_from(Bson(v)).map_err(|e| e.into())?,
                    );
                }
                Ok(m)
            }
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Document(...)` but found `{}`",
                inner
            ))
            .into()),
        }
    }
}

impl<T> TryFrom<Bson> for HashSet<T>
where
    T: Eq + Hash + TryFrom<Bson>,
    T::Error: Into<de::Error>,
{
    type Error = de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Array(a) => a
                .into_iter()
                .map(|x| T::try_from(Bson(x)).map_err(|e| e.into()))
                .collect(),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Array(...)` but found `{}`",
                inner
            ))
            .into()),
        }
    }
}

// FIXME: Blanket impls mess us up here we are not allowed to impl the below with current rust,
// fortunately we can work around this in the derive.
// https://github.com/rust-lang/rust/issues/31844
//impl<T> TryFrom<Bson> for Option<T>
//where
//    T: TryFrom<Bson>,
//    T::Error: Into<de::Error>,
//{
//    type Error = de::Error;
//    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
//        let inner = bson.0;
//        Ok(match inner {
//            bson::Bson::Null => None,
//            _ => Some(Bson(inner).try_into()?),
//        })
//    }
//}

impl<T> TryFrom<Bson> for Vec<T>
where
    T: TryFrom<Bson>,
    T::Error: Into<de::Error>,
{
    type Error = de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::Array(a) => a
                .into_iter()
                .map(|x| T::try_from(Bson(x)).map_err(|e| e.into()))
                .collect(),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::Array(...)` but found `{}`",
                inner
            ))
            .into()),
        }
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<Bson> for chrono::DateTime<chrono::Utc> {
    type Error = de::Error;
    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        let inner = bson.0;
        match inner {
            bson::Bson::DateTime(dt) => Ok(dt.into()),
            _ => Err(bson::de::Error::custom(format!(
                "invalid variant, expected `Bson::DateTime(...)` but found `{}`",
                inner
            ))
            .into()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bool_to_bson() {
        let v: bool = true;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Boolean(v));
    }

    #[test]
    fn char_to_bson() {
        let v: char = 'a';
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::String(v.to_string()));
    }

    #[test]
    fn f32_to_bson() {
        let v: f32 = 0.0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Double(0.0));
    }

    #[test]
    fn f64_to_bson() {
        let v: f64 = 0.0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Double(v));
    }

    #[test]
    fn i8_to_bson() {
        let v: i8 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(0));
    }

    #[test]
    fn i16_to_bson() {
        let v: i16 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(0));
    }

    #[test]
    fn i32_to_bson() {
        let v: i32 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(v));
    }

    #[test]
    fn i64_to_bson() {
        let v: i64 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int64(v));
    }

    #[test]
    fn u8_to_bson() {
        let v: u8 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(0));
    }

    #[test]
    fn u16_to_bson() {
        let v: u16 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(0));
    }

    #[test]
    fn u32_to_bson() {
        let v: u32 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int32(0));
    }

    #[test]
    fn u64_to_bson() {
        let v: u64 = 0;
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::Int64(0));
    }

    #[test]
    fn str_to_bson() {
        let v: &str = "abcd";
        let b = Bson::from(v).0;
        assert_eq!(b, bson::Bson::String(v.to_owned()));
    }

    #[test]
    fn string_to_bson() {
        let v: String = "abcd".to_owned();
        let b = Bson::from(v.clone()).0;
        assert_eq!(b, bson::Bson::String(v));
    }

    #[test]
    fn hashmap_to_bson() {
        let mut v: HashMap<String, String> = HashMap::new();
        v.insert("foo".to_owned(), "bar".to_owned());
        let b = Bson::try_from(v.clone()).unwrap().0;
        let mut l = 0;
        if let bson::Bson::Document(doc) = b {
            l = doc.len();
        }
        assert_eq!(v.len(), l);
    }

    #[test]
    fn hashset_to_bson() {
        let mut v: HashSet<String> = HashSet::new();
        v.insert("foo".to_owned());
        let b = Bson::try_from(v.clone()).unwrap().0;
        let mut l = 0;
        if let bson::Bson::Array(arr) = b {
            l = arr.len();
        }
        assert_eq!(v.len(), l);
    }

    #[test]
    fn vec_to_bson() {
        let v: Vec<String> = vec!["abcd".to_owned()];
        let b = Bson::try_from(v.clone()).unwrap().0;
        let mut l = 0;
        if let bson::Bson::Array(arr) = b {
            l = arr.len();
        }
        assert_eq!(v.len(), l);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_to_bson() {
        let v: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let b = Bson::try_from(v).unwrap().0;
        assert_eq!(b, bson::Bson::DateTime(v));
    }

    #[test]
    fn bson_to_bool() {
        let b = Bson(bson::Bson::Boolean(true));
        let v = bool::try_from(b).unwrap();
        assert_eq!(v, true);
    }

    #[test]
    fn bson_to_char() {
        let b = Bson(bson::Bson::String("a".to_owned()));
        let v = char::try_from(b).unwrap();
        assert_eq!(v, 'a');
    }

    #[test]
    fn bson_to_f32() {
        let b = Bson(bson::Bson::Double(0.0));
        let v = f32::try_from(b).unwrap();
        assert_eq!(v, 0.0);
    }

    #[test]
    fn bson_to_f64() {
        let b = Bson(bson::Bson::Double(0.0));
        let v = f64::try_from(b).unwrap();
        assert_eq!(v, 0.0);
    }

    #[test]
    fn bson_to_i8() {
        let b = Bson(bson::Bson::Int32(0));
        let v = i8::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_i16() {
        let b = Bson(bson::Bson::Int32(0));
        let v = i16::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_i32() {
        let b = Bson(bson::Bson::Int32(0));
        let v = i32::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_i64() {
        let b = Bson(bson::Bson::Int64(0));
        let v = i64::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_u8() {
        let b = Bson(bson::Bson::Int32(0));
        let v = u8::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_u16() {
        let b = Bson(bson::Bson::Int32(0));
        let v = u16::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_u32() {
        let b = Bson(bson::Bson::Int32(0));
        let v = u32::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_u64() {
        let b = Bson(bson::Bson::Int64(0));
        let v = u64::try_from(b).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn bson_to_string() {
        let b = Bson(bson::Bson::String("foo".to_owned()));
        let v = String::try_from(b).unwrap();
        assert_eq!(v, String::from("foo"));
    }

    #[test]
    fn bson_to_hashmap() {
        let mut doc: bson::Document = bson::Document::new();
        doc.insert("foo".to_owned(), "bar".to_owned());
        let b = Bson(bson::Bson::Document(doc));
        let m: HashMap<String, String> = HashMap::try_from(b).unwrap();
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn bson_to_hashset() {
        let b = Bson(bson::Bson::Array(vec![bson::Bson::String(
            "abcd".to_owned(),
        )]));
        let s: HashSet<String> = HashSet::try_from(b).unwrap();
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn bson_to_vec() {
        let b = Bson(bson::Bson::Array(vec![bson::Bson::String(
            "abcd".to_owned(),
        )]));
        let s: Vec<String> = Vec::try_from(b).unwrap();
        assert_eq!(s.len(), 1);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn bson_to_chrono() {
        let v: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let b = Bson(bson::Bson::DateTime(v));
        let dt = chrono::DateTime::try_from(b).unwrap();
        assert_eq!(dt, v);
    }
}
