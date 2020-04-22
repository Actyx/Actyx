/*
 * Copyright 2020 Actyx AG
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
#[cfg(feature = "dataflow")]
use abomonation::Abomonation;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{self, Formatter};
use std::ops::Deref;
use std::sync::Arc;

/// This is a helper type that allows an [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
/// to be included in a data structure that is serialized/deserialized with
/// Abomonation. See the source code for [`Semantics`](../event/struct.Semantics.html)
/// for an example use-case.
#[derive(Eq, PartialOrd, PartialEq, Ord, Hash)]
#[repr(transparent)]
pub struct ArcVal<T: ?Sized>(Arc<T>);

impl<T: Default> Default for ArcVal<T> {
    fn default() -> Self {
        Self(Arc::new(T::default()))
    }
}

impl<T: ?Sized> Deref for ArcVal<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.0
    }
}

impl<T: ?Sized> ArcVal<T> {
    pub fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.0)
    }

    pub fn as_arc(&self) -> &Arc<T> {
        &self.0
    }
}

impl<T: ?Sized> Clone for ArcVal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized, U: Sized> From<U> for ArcVal<T>
where
    U: Into<Arc<T>>,
{
    fn from(x: U) -> Self {
        Self(x.into())
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
        let arc: Arc<str> =
            str::from_utf8_unchecked(slice::from_raw_parts(mine.as_ptr(), *len)).into();
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

impl<'de, T: Deserialize<'de> + Sized> Deserialize<'de> for ArcVal<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ArcVal<T>, D::Error> {
        T::deserialize(deserializer).map(Self::from)
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
                Ok(ArcVal::from(v))
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
        let mut value = ArcVal::<String>::from("hello".to_owned());

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        // modify the string so that we can see whether decode built a new one
        unsafe { value.get_mut().unwrap().as_bytes_mut()[0] = 65 };
        let mut bytes2 = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes2).unwrap() };
        std::mem::drop(value);

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<String>>(&mut bytes).unwrap() };
        assert_eq!(*value, ArcVal::from("hello".to_owned()));
        assert!(bytes.is_empty());

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<String>>(&mut bytes2).unwrap() };
        assert_eq!(*value, ArcVal::from("Aello".to_owned()));
        assert!(bytes.is_empty());
    }

    #[test]
    #[cfg(feature = "dataflow")]
    pub fn must_work_for_str() {
        let mut value = ArcVal::<str>::from("hello");

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        // modify the string so that we can see whether decode built a new one
        unsafe { value.get_mut().unwrap().as_bytes_mut()[0] = 65 };
        let mut bytes2 = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes2).unwrap() };
        std::mem::drop(value);

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<str>>(&mut bytes).unwrap() };
        assert_eq!(*value, ArcVal::from("hello".to_owned()));
        assert!(bytes.is_empty());

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<str>>(&mut bytes2).unwrap() };
        assert_eq!(*value, ArcVal::from("Aello".to_owned()));
        assert!(bytes.is_empty());
    }

    #[test]
    #[cfg(feature = "dataflow")]
    #[allow(clippy::string_lit_as_bytes)]
    pub fn must_work_for_u8s() {
        let mut value = ArcVal::<[u8]>::from("hello".as_bytes());

        let mut bytes = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes).unwrap() };
        // modify the string so that we can see whether decode built a new one
        value.get_mut().unwrap()[0] = 65;
        let mut bytes2 = Vec::new();
        unsafe { abomonation::encode(&value, &mut bytes2).unwrap() };
        std::mem::drop(value);

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<[u8]>>(&mut bytes).unwrap() };
        assert_eq!(*value, ArcVal::from("hello".as_bytes()));
        assert!(bytes.is_empty());

        let (value, bytes) = unsafe { abomonation::decode::<ArcVal<[u8]>>(&mut bytes2).unwrap() };
        assert_eq!(*value, ArcVal::from("Aello".as_bytes()));
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
