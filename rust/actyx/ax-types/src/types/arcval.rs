use std::{hash::Hash, sync::Arc};

use intern_arc::{global::hash_interner, InternedHash};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    fmt::{self, Formatter},
    ops::Deref,
};

/// Interned and reference-counted immutable value
///
/// This type is a building block for handling large amounts of data with recurring heap-allocated
/// values, like strings for event types and entity names, but also binary data blocks that are
/// potentially loaded into memory multiple times.
///
/// ```
/// use ax_types::types::ArcVal;
///
/// let s: ArcVal<str> = ArcVal::clone_from_unsized("hello");
/// let b: ArcVal<[u8; 5]> = ArcVal::from_sized([49, 50, 51, 52, 53]);
/// let v: ArcVal<[u8]> = ArcVal::from_boxed(vec![49, 50, 51, 52, 53].into());
///
/// assert_eq!(&*s, "hello");
/// assert_eq!(&*b, b"12345");
/// assert_eq!(&*v, b"12345");
/// ```
///
/// # Caveat Emptor
///
/// It is obviously a very bad idea to intern objects that offer internal mutability, so don’t do that.
#[derive(Eq, PartialOrd, PartialEq, Ord, Hash)]
#[repr(transparent)]
pub struct ArcVal<T: Eq + Hash + ?Sized>(InternedHash<T>);

impl<T: Eq + Hash + Send + Sync + 'static> ArcVal<T> {
    pub fn from_sized(val: T) -> Self {
        Self(hash_interner().intern_sized(val))
    }
}

impl<T> ArcVal<T>
where
    T: ?Sized + Eq + Hash + Send + Sync + 'static + ToOwned,
    T::Owned: Into<Box<T>>,
    Arc<T>: for<'a> From<&'a T>,
{
    pub fn clone_from_unsized(val: &T) -> Self {
        Self(hash_interner().intern_ref(val))
    }
}

impl<T: ?Sized + Eq + Hash + Send + Sync + 'static> ArcVal<T> {
    pub fn from_boxed(val: Box<T>) -> Self {
        Self(hash_interner().intern_box(val))
    }
}

impl<T: Default + Eq + Hash + Send + Sync + 'static> Default for ArcVal<T> {
    fn default() -> Self {
        Self::from_sized(T::default())
    }
}

impl<T: Eq + Hash + ?Sized> Deref for ArcVal<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Eq + Hash + ?Sized> Clone for ArcVal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<Arc<T>> for ArcVal<T>
where
    T: ?Sized + Eq + Hash + Send + Sync + 'static + ToOwned,
    T::Owned: Into<Box<T>>,
{
    fn from(x: Arc<T>) -> Self {
        Self(hash_interner().intern_ref(&*x))
    }
}

impl Default for ArcVal<str> {
    fn default() -> Self {
        Self(hash_interner().intern_ref(""))
    }
}

#[cfg(any(test, feature = "arb"))]
impl quickcheck::Arbitrary for ArcVal<str> {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let s = String::arbitrary(g);
        ArcVal::from_boxed(s.into())
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let s = (*self.0).to_owned();
        Box::new(s.shrink().map(|s| ArcVal::from_boxed(s.into())))
    }
}

impl<T: fmt::Display + Eq + Hash + ?Sized> fmt::Display for ArcVal<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: fmt::Debug + Eq + Hash + ?Sized> fmt::Debug for ArcVal<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Serialize + Eq + Hash + Sized> Serialize for ArcVal<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for ArcVal<T>
where
    T: Deserialize<'de> + Sized + Eq + Hash + Send + Sync + 'static,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ArcVal<T>, D::Error> {
        T::deserialize(deserializer).map(Self::from_sized)
    }
}

impl Serialize for ArcVal<str> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArcVal<str> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ArcVal<str>, D::Error> {
        struct X();

        impl<'de> Visitor<'de> for X {
            type Value = ArcVal<str>;
            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(ArcVal::clone_from_unsized(v))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(ArcVal::from_boxed(v.into()))
            }
        }
        deserializer.deserialize_str(X())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    pub fn must_propagate_send_and_sync() {
        assert_send::<ArcVal<i32>>();
        assert_sync::<ArcVal<i32>>();
    }

    #[test]
    pub fn must_accept_str() {
        let arc: Arc<str> = "hello".into();
        let value: ArcVal<str> = ArcVal::from(arc);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"hello\"".to_owned());
    }
}
