use std::str::FromStr;

const HIERARCHY_SEPARATOR: char = '/';
const ROOT_SCOPE: char = '.';

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Scope [{0}] contains empty parts.")]
    InvalidScope(Scope),
    #[error("Scope [{0}] cannot be parsed.")]
    MalformedScope(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: we could replace our whole json pointer implementation with a library
// reducing the code we need to manage and so on
// however, this code is very pervasive in Actyx
// also, existing implementations of json pointer dont seem to have the
// diffing capability
#[derive(Ord, Eq, PartialOrd, PartialEq, Clone, Debug)]
pub struct Scope {
    pub tokens: Vec<String>,
}

impl FromStr for Scope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if s == "." {
            return Ok(Self::root());
        }

        let tokens: Vec<String> = s.split(HIERARCHY_SEPARATOR).map(|x| x.to_string()).collect();
        let scope = Self { tokens };
        if scope.tokens.iter().all(|t| !t.is_empty()) {
            Ok(scope)
        } else {
            Err(Error::InvalidScope(scope))
        }
    }
}

impl TryFrom<&str> for Scope {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        FromStr::from_str(value)
    }
}

impl TryFrom<String> for Scope {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        FromStr::from_str(&value)
    }
}

impl From<Scope> for String {
    fn from(scope: Scope) -> Self {
        scope.to_string()
    }
}

/// Enables using scopes for String-based lookup Maps
impl From<&Scope> for String {
    fn from(scope: &Scope) -> Self {
        scope.to_string()
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s: String = if self.is_root() {
            ROOT_SCOPE.to_string()
        } else {
            self.tokens.join(&HIERARCHY_SEPARATOR.to_string())
        };
        fmt.write_str(&s)
    }
}
pub struct ScopeIter<'a, I: Iterator<Item = &'a String>> {
    iter: I,
    len: usize,
}
impl<'a, I: Iterator<Item = &'a String>> Iterator for ScopeIter<'a, I> {
    type Item = &'a String;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
impl<'a, I: Iterator<Item = &'a String>> ExactSizeIterator for ScopeIter<'a, I> {
    fn len(&self) -> usize {
        self.len
    }
}

impl serde::ser::Serialize for Scope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> serde::de::Deserialize<'de> for Scope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(ScopeVisitor)
    }
}

struct ScopeVisitor;
impl<'de> serde::de::Visitor<'de> for ScopeVisitor {
    type Value = Scope;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a scope string")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Scope::from_str(s).map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))
    }
}

#[allow(clippy::len_without_is_empty)]
impl Scope {
    /// Return the root scope.
    pub fn root() -> Self {
        Self { tokens: vec![] }
    }

    pub fn append(&self, other: &Self) -> Self {
        let mut new_tokens = self.tokens.clone();
        let mut other_tokens = other.tokens.clone();
        new_tokens.append(&mut other_tokens);
        Self { tokens: new_tokens }
    }
    pub fn is_root(&self) -> bool {
        self.tokens.is_empty()
    }
    pub fn iter(&self) -> ScopeIter<'_, std::slice::Iter<'_, String>> {
        ScopeIter {
            iter: self.tokens.iter(),
            len: self.tokens.len(),
        }
    }
    pub fn drop_first(&self) -> Self {
        let mut t = self.tokens.clone();
        let tokens = if t.is_empty() {
            t
        } else {
            t.drain(1..self.tokens.len()).collect()
        };
        Self { tokens }
    }
    pub fn drop_last(&self) -> Self {
        let mut tokens = self.tokens.clone();
        tokens.pop();
        Self { tokens }
    }
    pub fn pop_mut(&mut self) -> Option<Self> {
        self.tokens.pop().map(|item| Self { tokens: vec![item] })
    }
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
    pub fn starts_with(&self, other: &Scope) -> bool {
        if other.is_root() {
            return true;
        }
        let self_tokens = self.iter();
        let other_tokens = other.iter();

        // `other` is more specific than `self`
        if other_tokens.len() > self_tokens.len() {
            return false;
        }
        !self_tokens.zip(other_tokens).any(|(a, b)| a != b)
    }
    /// Removes a common prefix from self.
    pub fn diff(&self, other: &Self) -> Option<Self> {
        let tokens: Vec<String> = self
            .iter()
            .zip(other.iter())
            .filter(|(x1, x2)| x1 != x2)
            .map(|(x1, _)| x1)
            .chain(self.iter().skip(other.len()))
            .cloned()
            .collect();
        if !tokens.is_empty() {
            Some(Self { tokens })
        } else {
            None
        }
    }
    pub fn as_json_ptr(&self) -> String {
        if self.is_root() {
            // Root pointer is the empty string, not "."
            "".to_owned()
        } else {
            HIERARCHY_SEPARATOR.to_string() + &self.tokens.join(&HIERARCHY_SEPARATOR.to_string())
        }
    }
    pub fn first(&self) -> Option<String> {
        self.tokens.get(0).cloned()
    }
    pub fn split_first(&self) -> Option<(Self, Self)> {
        if !self.is_root() {
            let (head, tail) = self.tokens.split_at(1);
            Some((Self { tokens: head.to_vec() }, Self { tokens: tail.to_vec() }))
        } else {
            None
        }
    }
    pub fn split_last(&self) -> Option<(Self, Self)> {
        if !self.is_root() {
            let (init, last) = self.tokens.split_at(self.tokens.len() - 1);
            Some((Self { tokens: init.to_vec() }, Self { tokens: last.to_vec() }))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn root() {
        let root_scope = Scope::root();
        assert!(root_scope.is_root());
        assert_eq!(ROOT_SCOPE.to_string(), root_scope.to_string());
    }

    #[test]
    fn empty() {
        for scope in ["", "/", "//", "/a", "a/", "/a/", "a//b"] {
            assert!(Scope::try_from(scope).is_err());
        }
    }

    #[test]
    fn to_string() {
        let scope = Scope::try_from("a/b/c").unwrap();
        assert_eq!(&scope.to_string(), "a/b/c");
    }

    #[test]
    fn append() {
        let test_input = [
            (".", ".", "."),
            (".", "a", "a"),
            ("a", ".", "a"),
            ("a/b", "c/d", "a/b/c/d"),
        ];
        for (input, other, expected) in test_input {
            let head = Scope::try_from(input).unwrap();
            let tail = Scope::try_from(other).unwrap();
            assert_eq!(
                head.append(&tail),
                Scope::try_from(expected).unwrap(),
                "input was ({}, {})",
                input,
                other
            );
        }
    }

    #[test]
    fn diff() {
        let test_input = [
            ("a", "a", None),
            ("a", "a/b", None),
            ("a/a", "a/b", Some("a")),
            ("a/b", "a", Some("b")),
            ("a/b", "a/b", None),
            ("a/b/c/d", ".", Some("a/b/c/d")),
            (".", "a/b/c/d", None),
            ("a/b/c/d", "a/b/e/f", Some("c/d")),
            ("a/b/c/d", "a/b/a/b", Some("c/d")),
            ("a.b.c", "a.b.c", None),
            ("a.b.c/d/e", "a.b.c", Some("d/e")),
        ];
        for (input, other, diff_result) in test_input {
            let a = Scope::try_from(input).unwrap();
            let b = Scope::try_from(other).unwrap();
            let diff = a.diff(&b);
            let expected_diff = diff_result.map(|diff| Scope::try_from(diff).unwrap());
            assert_eq!(diff, expected_diff, "input was: ({}, {})", input, other)
        }
    }

    #[test]
    fn starts_with() {
        let scope = Scope::try_from("first/scope/really").unwrap();
        let first_scope = Scope::try_from("first").unwrap();
        let other_scope = Scope::try_from("other").unwrap();
        // Check that start_with validates the right and wrong scopes correctly
        assert!(scope.starts_with(&first_scope));
        assert!(!scope.starts_with(&other_scope));
        // Every scope starts at the root scope
        assert!(scope.starts_with(&Scope::root()));
    }

    #[test]
    fn first() {
        let test_input = [(".", None), ("a", Some("a")), ("a/b/c/d", Some("a"))];
        for (input, expected) in test_input {
            let actual = Scope::try_from(input).unwrap().first();
            assert_eq!(actual, expected.map(ToString::to_string), "input was: [{}]", input)
        }
    }

    #[test]
    fn split_first() {
        let test_input = [(".", None), ("a", Some(("a", "."))), ("a/b/c/d", Some(("a", "b/c/d")))];
        for (input, expected) in test_input {
            let actual = Scope::try_from(input).unwrap().split_first();
            assert_eq!(
                actual,
                expected.map(move |(head, tail)| (Scope::try_from(head).unwrap(), Scope::try_from(tail).unwrap())),
                "input was: [{}]",
                input
            )
        }
    }

    #[test]
    fn split_last() {
        let test_input = [(".", None), ("a", Some((".", "a"))), ("a/b/c/d", Some(("a/b/c", "d")))];
        for (input, expected) in test_input {
            let actual = Scope::try_from(input).unwrap().split_last();
            assert_eq!(
                actual,
                expected.map(|(init, last)| (Scope::try_from(init).unwrap(), Scope::try_from(last).unwrap())),
                "input was: [{}]",
                input
            )
        }
    }

    #[test]
    fn serialize() {
        assert_eq!(
            serde_json::to_value(Scope::try_from("a/b/c").unwrap()).unwrap(),
            serde_json::json!("a/b/c")
        );
    }

    #[test]
    fn deserialize() {
        let scope: Scope = serde_json::from_value(serde_json::json!("a/b/c")).unwrap();
        assert_eq!(scope, Scope::try_from("a/b/c").unwrap());
    }
}
