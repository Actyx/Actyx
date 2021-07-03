mod parser;
mod render;

use crate::{language::render::render_tag_expr, tags::Tag, AppId, LamportTimestamp, Timestamp};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Query {
    pub from: TagExpr,
    pub ops: Vec<Operation>,
}
mod query_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(Vec<SimpleExpr>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagExpr {
    Or(Box<(TagExpr, TagExpr)>),
    And(Box<(TagExpr, TagExpr)>),
    Atom(TagAtom),
}

impl TagExpr {
    pub fn and(self, other: TagExpr) -> Self {
        TagExpr::And(Box::new((self, other)))
    }
    pub fn or(self, other: TagExpr) -> Self {
        TagExpr::Or(Box::new((self, other)))
    }
}

impl std::fmt::Display for TagExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render_tag_expr(f, self, None)
    }
}

impl std::ops::BitOr for TagExpr {
    type Output = TagExpr;
    fn bitor(self, that: Self) -> Self {
        self.or(that)
    }
}

impl std::ops::BitAnd for TagExpr {
    type Output = TagExpr;
    fn bitand(self, that: Self) -> Self {
        self.and(that)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagAtom {
    Tag(Tag),
    AllEvents,
    IsLocal,
    FromTime(Timestamp),
    ToTime(Timestamp),
    FromLamport(LamportTimestamp),
    ToLamport(LamportTimestamp),
    AppId(AppId),
}

impl TagAtom {
    pub fn tag(&self) -> Option<&Tag> {
        if let Self::Tag(tag) = self {
            Some(tag)
        } else {
            None
        }
    }
    pub fn is_local(&self) -> bool {
        matches!(self, Self::IsLocal)
    }
}

// this will obviously need to be implemented for real sometime, with arbitrary precision
#[derive(Debug, Clone)]
pub enum Number {
    Decimal(f64),
    Natural(u64),
}
mod number_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Index {
    Ident(String),
    Number(u64),
    Expr(SimpleExpr),
}

impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Index::Ident(s) => write!(f, ".{}", s),
            Index::Number(n) => write!(f, ".{}", n),
            Index::Expr(e) => write!(f, ".[{}]", e),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Indexing {
    pub head: Box<SimpleExpr>,
    pub tail: Vec<Index>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Object {
    pub props: Vec<(String, SimpleExpr)>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Array {
    pub items: Vec<SimpleExpr>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum SimpleExpr {
    Var(String),
    Indexing(Indexing),
    Number(Number),
    String(String),
    Object(Object),
    Array(Array),
    Null,
    Bool(bool),
    Add(Box<(SimpleExpr, SimpleExpr)>),
    Sub(Box<(SimpleExpr, SimpleExpr)>),
    Mul(Box<(SimpleExpr, SimpleExpr)>),
    Div(Box<(SimpleExpr, SimpleExpr)>),
    Mod(Box<(SimpleExpr, SimpleExpr)>),
    Pow(Box<(SimpleExpr, SimpleExpr)>),
    And(Box<(SimpleExpr, SimpleExpr)>),
    Or(Box<(SimpleExpr, SimpleExpr)>),
    Not(Box<SimpleExpr>),
    Xor(Box<(SimpleExpr, SimpleExpr)>),
    Lt(Box<(SimpleExpr, SimpleExpr)>),
    Le(Box<(SimpleExpr, SimpleExpr)>),
    Gt(Box<(SimpleExpr, SimpleExpr)>),
    Ge(Box<(SimpleExpr, SimpleExpr)>),
    Eq(Box<(SimpleExpr, SimpleExpr)>),
    Ne(Box<(SimpleExpr, SimpleExpr)>),
}

impl std::fmt::Display for SimpleExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render::render_simple_expr(f, self)
    }
}

#[allow(clippy::clippy::should_implement_trait)]
impl SimpleExpr {
    pub fn add(self, other: SimpleExpr) -> Self {
        SimpleExpr::Add(Box::new((self, other)))
    }
    pub fn sub(self, other: SimpleExpr) -> Self {
        SimpleExpr::Sub(Box::new((self, other)))
    }
    pub fn mul(self, other: SimpleExpr) -> Self {
        SimpleExpr::Mul(Box::new((self, other)))
    }
    pub fn div(self, other: SimpleExpr) -> Self {
        SimpleExpr::Div(Box::new((self, other)))
    }
    pub fn modulo(self, other: SimpleExpr) -> Self {
        SimpleExpr::Mod(Box::new((self, other)))
    }
    pub fn pow(self, other: SimpleExpr) -> Self {
        SimpleExpr::Pow(Box::new((self, other)))
    }
    pub fn and(self, other: SimpleExpr) -> Self {
        SimpleExpr::And(Box::new((self, other)))
    }
    pub fn or(self, other: SimpleExpr) -> Self {
        SimpleExpr::Or(Box::new((self, other)))
    }
    pub fn xor(self, other: SimpleExpr) -> Self {
        SimpleExpr::Xor(Box::new((self, other)))
    }
    pub fn lt(self, other: SimpleExpr) -> Self {
        SimpleExpr::Lt(Box::new((self, other)))
    }
    pub fn le(self, other: SimpleExpr) -> Self {
        SimpleExpr::Le(Box::new((self, other)))
    }
    pub fn gt(self, other: SimpleExpr) -> Self {
        SimpleExpr::Gt(Box::new((self, other)))
    }
    pub fn ge(self, other: SimpleExpr) -> Self {
        SimpleExpr::Ge(Box::new((self, other)))
    }
    pub fn eq(self, other: SimpleExpr) -> Self {
        SimpleExpr::Eq(Box::new((self, other)))
    }
    pub fn ne(self, other: SimpleExpr) -> Self {
        SimpleExpr::Ne(Box::new((self, other)))
    }
}

#[cfg(test)]
mod for_tests {
    use super::*;

    impl Query {
        pub fn new(from: TagExpr) -> Self {
            Self { from, ops: vec![] }
        }
        pub fn push(&mut self, op: Operation) {
            self.ops.push(op);
        }
        pub fn with_op(self, op: Operation) -> Self {
            let Self { from, mut ops } = self;
            ops.push(op);
            Self { from, ops }
        }
    }

    impl TagAtom {
        pub fn and(self, other: TagAtom) -> TagExpr {
            TagExpr::And(Box::new((TagExpr::Atom(self), TagExpr::Atom(other))))
        }
        pub fn or(self, other: TagAtom) -> TagExpr {
            TagExpr::Or(Box::new((TagExpr::Atom(self), TagExpr::Atom(other))))
        }
    }

    pub trait ToIndex {
        fn into(&self) -> Index;
    }
    impl ToIndex for &str {
        fn into(&self) -> Index {
            Index::Ident((*self).to_owned())
        }
    }
    impl ToIndex for u64 {
        fn into(&self) -> Index {
            Index::Number(*self)
        }
    }

    impl Indexing {
        pub fn with(head: impl Into<String>, tail: &[&dyn ToIndex]) -> SimpleExpr {
            SimpleExpr::Indexing(Self {
                head: Box::new(SimpleExpr::Var(head.into())),
                tail: tail.iter().map(|x| (*x).into()).collect(),
            })
        }
        pub fn ident(ident: impl Into<String>) -> SimpleExpr {
            SimpleExpr::Indexing(Self {
                head: Box::new(SimpleExpr::Var(ident.into())),
                tail: vec![],
            })
        }
    }

    impl Object {
        pub fn with(props: &[(&str, SimpleExpr)]) -> SimpleExpr {
            SimpleExpr::Object(Object {
                props: props.iter().map(|(x, e)| ((*x).to_owned(), e.clone())).collect(),
            })
        }
    }

    impl Array {
        pub fn with(items: &[SimpleExpr]) -> SimpleExpr {
            SimpleExpr::Array(Array { items: items.to_vec() })
        }
    }
}
