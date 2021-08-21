use std::{convert::TryFrom, ops::Deref, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Var(pub(crate) String);

#[derive(Debug, Clone)]
pub struct NoVar;
impl std::error::Error for NoVar {}
impl std::fmt::Display for NoVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a variable name must start with a lowercase letter and can only contain lowercase letters, numbers, and underscore")
    }
}

impl TryFrom<&str> for Var {
    type Error = NoVar;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value).map_err(|_| NoVar)
    }
}

impl TryFrom<String> for Var {
    type Error = NoVar;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl Deref for Var {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Var {
    /// For internal use only
    ///
    /// If you create invalid variables and serialize the expression, be prepared for breakage.
    pub fn internal(s: String) -> Self {
        Self(s)
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Var {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        use once_cell::sync::OnceCell;
        static CHOICES: OnceCell<Vec<char>> = OnceCell::new();

        let choices = CHOICES.get_or_init(|| {
            ('a'..='z')
                .chain('0'..='9')
                .chain(std::iter::once('_'))
                .collect::<Vec<_>>()
        });
        let mut first = true;
        let s = Vec::<bool>::arbitrary(g)
            .into_iter()
            .map(|_| {
                if first {
                    first = false;
                    *g.choose(&choices[0..26]).unwrap()
                } else {
                    *g.choose(choices).unwrap()
                }
            })
            .collect::<String>();
        if s.is_empty() {
            Self("x".into())
        } else {
            Self(s)
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().filter_map(|v| Self::from_str(&v).ok()))
    }
}
