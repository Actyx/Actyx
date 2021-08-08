#![allow(dead_code)]
use std::{convert::TryFrom, ops::Deref, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonEmptyVec<T>(Arc<[T]>);

#[derive(Debug, Clone)]
pub struct NoElements;
impl std::error::Error for NoElements {}
impl std::fmt::Display for NoElements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot construct NonEmptyVec without elements")
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptyVec<T> {
    type Error = NoElements;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(NoElements)
        } else {
            Ok(Self(value.into()))
        }
    }
}

impl<T> Deref for NonEmptyVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
impl<T: quickcheck::Arbitrary + Clone + 'static> quickcheck::Arbitrary for NonEmptyVec<T> {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut v = Vec::<T>::arbitrary(g);
        loop {
            if !v.is_empty() {
                break;
            }
            v = Vec::<T>::arbitrary(g);
        }
        Self(v.into())
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            self.0
                .to_vec()
                .shrink()
                .filter(|v| !v.is_empty())
                .map(|v| Self(v.into())),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonEmptyString(String);

#[derive(Debug, Clone)]
pub struct NoChars;
impl std::error::Error for NoChars {}
impl std::fmt::Display for NoChars {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot construct NonEmptyString without characters")
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = NoElements;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(NoElements)
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = NoElements;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(NoElements)
        } else {
            Ok(Self(value.to_owned()))
        }
    }
}

impl Deref for NonEmptyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl NonEmptyString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for NonEmptyString {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut v = String::arbitrary(g);
        loop {
            if !v.is_empty() {
                break;
            }
            v = String::arbitrary(g);
        }
        Self(v)
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().filter(|v| !v.is_empty()).map(Self))
    }
}
