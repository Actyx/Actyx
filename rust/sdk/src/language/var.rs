use std::{convert::TryFrom, ops::Deref};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Var(String);

#[derive(Debug, Clone)]
pub struct NoVar;
impl std::error::Error for NoVar {}
impl std::fmt::Display for NoVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a variable name must start with a lowercase letter and can only contain lowercase letters, numbers, and underscore")
    }
}

fn is_var(s: &str) -> bool {
    s == "_"
        || s.chars().next().into_iter().any(|c| c.is_lowercase())
            && s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
}

impl TryFrom<&str> for Var {
    type Error = NoVar;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if is_var(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(NoVar)
        }
    }
}

impl TryFrom<String> for Var {
    type Error = NoVar;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if is_var(&value) {
            Ok(Self(value))
        } else {
            Err(NoVar)
        }
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
        Box::new(self.0.shrink().filter(|v| is_var(v)).map(Self))
    }
}
