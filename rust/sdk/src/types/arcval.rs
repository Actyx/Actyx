/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::{hash::Hash, sync::Arc};

#[cfg(feature = "dataflow")]
use abomonation::Abomonation;
use intern_arc::*;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{self, Formatter};
use std::ops::Deref;

/// Helper macro to create interned string types
///
/// ```
/// use actyxos_sdk::arcval_scalar;
///
/// arcval_scalar! {
///     /// some docs
///     pub struct Name(str);
/// }
///
/// arcval_scalar! { struct Private(str) }
///
/// let n: Name = Name::from("Bob");
/// let p: Private = Private::from("x".to_owned());
/// ```
///
/// The declared wrapper struct derives instances for the standard library traits
/// Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash. You can add more with the
/// usual `#[derive()]` attribute.
#[macro_export]
macro_rules! arcval_scalar {
    ($($(#[$attr:meta])* $vis:vis struct $id:ident(str)$(;)?)*) => {
        $(
            $(#[$attr])*
            #[repr(transparent)]
            #[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
            $vis struct $id($crate::types::ArcVal<str>);
            impl $id {
                pub fn as_arc(&self) -> ::std::sync::Arc<str> {
                    self.0.as_arc().clone()
                }
            }
            impl ::std::ops::Deref for $id {
                type Target = str;
                fn deref(&self) -> &str {
                    &*self.0
                }
            }
            impl From<String> for $id {
                fn from(s: String) -> Self {
                    Self($crate::types::ArcVal::from_boxed(s.into()))
                }
            }
            impl<'a> From<&'a str> for $id {
                fn from(s: &'a str) -> Self {
                    Self($crate::types::ArcVal::clone_from_unsized(s))
                }
            }
            impl From<::std::sync::Arc<str>> for $id {
                fn from(s: ::std::sync::Arc<str>) -> Self {
                    Self($crate::types::ArcVal::from(s))
                }
            }
            impl From<$crate::types::ArcVal<str>> for $id {
                fn from(s: $crate::types::ArcVal<str>) -> Self {
                    Self(s)
                }
            }
            impl Into<$crate::types::ArcVal<str>> for $id {
                fn into(self) -> $crate::types::ArcVal<str> {
                    self.0
                }
            }
            impl ::std::fmt::Display for $id {
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    ::std::write!(f, "{}", self.0)
                }
            }
            impl Default for $id {
                fn default() -> Self {
                    Self::from("")
                }
            }
        )*
    };
}

/// Interned and reference-counted immutable value
///
/// This type is a building block for handling large amounts of data with recurring heap-allocated
/// values, like strings for event types and entity names, but also binary data blocks that are
/// potentially loaded into memory multiple times. The [`arcval_scalar!`](../macro.arcval_scalar.html)
/// macro makes it easy to tag data to denote different kinds of objects.
///
/// This also serves as a helper type that allows an [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
/// to be included in a data structure that is serialized/deserialized with
/// [`Abomonation`](https://docs.rs/abomonation).
///
/// ```
/// use actyxos_sdk::types::ArcVal;
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
pub struct ArcVal<T: ?Sized>(Arc<T>);

impl<T: Eq + Hash + Ord + Send + Sync + 'static> ArcVal<T> {
    pub fn from_sized(val: T) -> Self {
        Self(intern(val))
    }
}

impl<T> ArcVal<T>
where
    T: ?Sized + Eq + Hash + Ord + Send + Sync + 'static,
    Arc<T>: for<'a> From<&'a T>,
{
    pub fn clone_from_unsized(val: &T) -> Self {
        Self(intern_unsized(val))
    }
}

impl<T: ?Sized + Eq + Hash + Ord + Send + Sync + 'static> ArcVal<T> {
    pub fn from_boxed(val: Box<T>) -> Self {
        Self(intern_boxed(val))
    }
}

impl<T: ?Sized> ArcVal<T> {
    pub fn as_arc(&self) -> &Arc<T> {
        &self.0
    }
}

impl<T: Default + Eq + Hash + Ord + Send + Sync + 'static> Default for ArcVal<T> {
    fn default() -> Self {
        Self::from_sized(T::default())
    }
}

impl<T: ?Sized> Deref for ArcVal<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.0
    }
}

impl<T: ?Sized> Clone for ArcVal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<Arc<T>> for ArcVal<T>
where
    T: ?Sized + Eq + Hash + Ord + Send + Sync + 'static,
{
    fn from(x: Arc<T>) -> Self {
        Self(intern_arc(x))
    }
}

impl Default for ArcVal<str> {
    fn default() -> Self {
        Self(intern_unsized(""))
    }
}

/// This abomination only works if the underlying bytes are known in number (Sized),
/// can be moved around in memory (Unpin) and do no hold on to other things than the
/// underlying bytes ('static).
#[cfg(feature = "dataflow")]
impl<T: Abomonation + 'static + Sized + Unpin> Abomonation for ArcVal<T> {
    unsafe fn entomb<W: std::io::Write>(&self, write: &mut W) -> std::io::Result<()> {
        /* Since the value T has not yet been seen by abomonate (it is behind a pointer)
         * we need to fully encode it.
         */
        abomonation::encode(self.deref(), write)
    }
    unsafe fn exhume<'a, 'b>(&'a mut self, bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
        use std::{mem, ptr};
        /* The idea here is to construct a new Arc<T> from the entombed bytes.
         * The state of this ArcVal upon entry of this function contains only an invalid
         * pointer to an ArcInner that we need to dispose of without trying to run
         * its destructor (which would panic).
         */
        let (value, bytes) = abomonation::decode::<T>(bytes)?;
        // value is just a reference to the first part of old bytes, so move it into a new Arc
        let arc = Arc::new(ptr::read(value));
        // now swap the fresh arc into its place ...
        let garbage = mem::replace(&mut self.0, arc);
        // ... and forget about the old one
        mem::forget(garbage);
        Some(bytes)
    }
    fn extent(&self) -> usize {
        std::mem::size_of::<T>() + self.deref().extent()
    }
}

#[cfg(feature = "dataflow")]
impl Abomonation for ArcVal<str> {
    unsafe fn entomb<W: std::io::Write>(&self, write: &mut W) -> std::io::Result<()> {
        let len = self.0.len();
        let buf = self.0.as_ptr();
        abomonation::encode(&len, write)?;
        let buf = std::slice::from_raw_parts(buf, len);
        write.write_all(buf)
    }
    unsafe fn exhume<'a, 'b>(&'a mut self, bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
        use std::{mem, slice, str};
        let (len, bytes) = abomonation::decode::<usize>(bytes)?;
        if bytes.len() < *len {
            return None;
        }
        let (mine, bytes) = bytes.split_at_mut(*len);
        let arc: Arc<str> = str::from_utf8_unchecked(slice::from_raw_parts(mine.as_ptr(), *len)).into();
        let garbage = mem::replace(&mut self.0, arc);
        mem::forget(garbage);
        Some(bytes)
    }
    fn extent(&self) -> usize {
        std::mem::size_of::<usize>() + self.deref().len()
    }
}

#[cfg(feature = "dataflow")]
impl Abomonation for ArcVal<[u8]> {
    unsafe fn entomb<W: std::io::Write>(&self, write: &mut W) -> std::io::Result<()> {
        let len = self.0.len();
        abomonation::encode(&len, write)?;
        write.write_all(&self.0)
    }
    unsafe fn exhume<'a, 'b>(&'a mut self, bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
        use std::{mem, slice};
        let (len, bytes) = abomonation::decode::<usize>(bytes)?;
        if bytes.len() < *len {
            return None;
        }
        let (mine, bytes) = bytes.split_at_mut(*len);
        let arc: Arc<[u8]> = slice::from_raw_parts(mine.as_ptr(), *len).into();
        let garbage = mem::replace(&mut self.0, arc);
        mem::forget(garbage);
        Some(bytes)
    }
    fn extent(&self) -> usize {
        std::mem::size_of::<usize>() + self.deref().len()
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for ArcVal<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for ArcVal<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Serialize + Sized> Serialize for ArcVal<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for ArcVal<T>
where
    T: Deserialize<'de> + Sized + Eq + Hash + Ord + Send + Sync + 'static,
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
    #[cfg(feature = "dataflow")]
    pub fn must_exhume() {
        let value = ArcVal::from_sized("hello".to_owned());

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        assert_eq!(&bytes[bytes.len() - 5..], b"hello");

        // modify the bytes to see that deserialization uses them and not the pointer
        let pos = bytes.len() - 4;
        bytes[pos] = b'a';

        let (value2, bytes) = unsafe { abomonation::decode::<ArcVal<String>>(&mut bytes).unwrap() };
        assert_eq!(value2.as_ref(), "hallo".to_owned());
        assert!(bytes.is_empty());
    }

    #[test]
    #[cfg(feature = "dataflow")]
    pub fn must_work_for_str() {
        let value = ArcVal::clone_from_unsized("hello");

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        assert_eq!(&bytes[bytes.len() - 5..], b"hello");

        // modify the bytes to see that deserialization uses them and not the pointer
        let pos = bytes.len() - 4;
        bytes[pos] = b'a';

        let (value2, bytes) = unsafe { abomonation::decode::<ArcVal<str>>(&mut bytes).unwrap() };
        assert_eq!(value2.as_ref(), "hallo".to_owned());
        assert!(bytes.is_empty());
    }

    #[test]
    #[cfg(feature = "dataflow")]
    #[allow(clippy::string_lit_as_bytes)]
    pub fn must_work_for_u8s() {
        let value: ArcVal<[u8]> = ArcVal::clone_from_unsized(&*b"hello");

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        assert_eq!(&bytes[bytes.len() - 5..], b"hello");

        // modify the bytes to see that deserialization uses them and not the pointer
        let pos = bytes.len() - 4;
        bytes[pos] = b'a';

        let (value2, bytes) = unsafe { abomonation::decode::<ArcVal<[u8]>>(&mut bytes).unwrap() };
        assert_eq!(value2.as_ref(), b"hallo");
        assert!(bytes.is_empty());
    }

    #[test]
    pub fn must_accept_str() {
        let arc: Arc<str> = "hello".into();
        let value: ArcVal<str> = ArcVal::from(arc);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"hello\"".to_owned());
    }
}
