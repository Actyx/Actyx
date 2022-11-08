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

#[derive(Ord, Eq, PartialOrd, PartialEq, Clone, Debug)]
pub struct Scope {
    pub tokens: Vec<String>,
}

impl std::convert::TryFrom<String> for Scope {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        if value == "." {
            return Ok(Self { tokens: vec![] });
        }

        let tokens: Vec<String> = value.split(HIERARCHY_SEPARATOR).map(|x| x.to_string()).collect();
        let scope = Self { tokens };
        if scope.tokens.iter().all(|t| !t.is_empty()) {
            Ok(scope)
        } else {
            Err(Error::InvalidScope(scope))
        }
    }
}

impl std::str::FromStr for Scope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        use std::convert::TryFrom;
        Scope::try_from(s.to_string())
    }
}

#[cfg(test)]
impl std::convert::From<&'static str> for Scope {
    fn from(value: &'static str) -> Self {
        use std::convert::TryInto;
        value.to_string().try_into().unwrap()
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
        use std::str::FromStr;
        Scope::from_str(s).map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))
    }
}

#[allow(clippy::len_without_is_empty)]
impl Scope {
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
        vec!["", "/", "//", "/a", "a/", "/a/", "a//b"]
            .into_iter()
            .for_each(|s| {
                assert!(std::panic::catch_unwind(|| Scope::from(s)).is_err());
            });
    }

    #[test]
    fn to_string() {
        let scope = "a/b/c";
        assert_eq!(Scope::from(scope).to_string(), scope)
    }

    #[test]
    fn into_string() {
        let scope = "a/b/c";
        let str: String = Scope::from("a/b/c").into();
        assert_eq!(str, scope)
    }

    #[test]
    fn ref_into_string() {
        let str: String = (&Scope::from("a/b/c")).into();
        assert_eq!(str, "a/b/c")
    }

    #[test]
    fn append() {
        vec![
            (".", ".", "."),
            (".", "a", "a"),
            ("a", ".", "a"),
            ("a/b", "c/d", "a/b/c/d"),
        ]
        .into_iter()
        .for_each(|(input, other, expected)| {
            assert_eq!(
                Scope::from(input).append(&Scope::from(other)),
                Scope::from(expected),
                "input was ({}, {})",
                input,
                other
            );
        });
    }

    #[test]
    fn diff() {
        vec![
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
        ]
        .into_iter()
        .for_each(|(input, other, expected)| {
            assert_eq!(
                Scope::from(input).diff(&Scope::from(other)),
                expected.map(Scope::from),
                "input was: ({}, {})",
                input,
                other
            )
        });
    }

    #[test]
    fn starts_with() {
        let my_first_scope: Scope = "first/scope/really".into();
        let root_scope = Scope::root();
        assert!(my_first_scope.starts_with(&"first".into()));
        assert!(!Scope::from("first").starts_with(&my_first_scope));
        assert!(!my_first_scope.starts_with(&"whatever".into()));
        assert!(my_first_scope.starts_with(&root_scope));
        assert!(!root_scope.starts_with(&my_first_scope));
    }

    #[test]
    fn first() {
        vec![(".", None), ("a", Some("a")), ("a/b/c/d", Some("a"))]
            .into_iter()
            .for_each(|(input, expected)| {
                let actual = Scope::from(input).first();
                assert_eq!(actual, expected.map(String::from), "input was: [{}]", input)
            });
    }

    #[test]
    fn split_first() {
        vec![(".", None), ("a", Some(("a", "."))), ("a/b/c/d", Some(("a", "b/c/d")))]
            .into_iter()
            .for_each(|(input, expected)| {
                let actual = Scope::from(input).split_first();
                assert_eq!(
                    actual,
                    expected.map(move |(head, tail)| (head.into(), tail.into())),
                    "input was: [{}]",
                    input
                )
            });
    }

    #[test]
    fn split_last() {
        vec![(".", None), ("a", Some((".", "a"))), ("a/b/c/d", Some(("a/b/c", "d")))]
            .into_iter()
            .for_each(|(input, expected)| {
                let actual = Scope::from(input).split_last();
                assert_eq!(
                    actual,
                    expected.map(|(init, last)| (init.into(), last.into())),
                    "input was: [{}]",
                    input
                )
            });
    }

    #[test]
    fn serialize() {
        assert_eq!(
            serde_json::to_value(Scope::from("a/b/c")).unwrap(),
            serde_json::json!("a/b/c")
        );
    }

    #[test]
    fn deserialize() {
        let scope: Scope = serde_json::from_value(serde_json::json!("a/b/c")).unwrap();
        assert_eq!(scope, Scope::from("a/b/c"));
    }
}
