mod non_empty;
mod parser;
mod render;

use self::{non_empty::NonEmptyVec, render::render_tag_expr};
use crate::{tags::Tag, AppId, LamportTimestamp, Timestamp};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Query {
    pub features: Vec<String>,
    pub from: TagExpr,
    pub ops: Vec<Operation>,
}
mod query_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(NonEmptyVec<SimpleExpr>),
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
pub enum Num {
    Decimal(f64),
    Natural(u64),
}
mod number_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Index {
    String(String),
    Number(u64),
    Expr(SimpleExpr),
}

fn is_ident(s: &str) -> bool {
    s == "_"
        || !s.is_empty()
            && s.chars().next().unwrap().is_lowercase()
            && s.chars().all(|c: char| c.is_lowercase() || c.is_numeric() || c == '_')
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Ind {
    pub head: Box<SimpleExpr>,
    pub tail: NonEmptyVec<Index>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Obj {
    pub props: Vec<(Index, SimpleExpr)>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Arr {
    pub items: Vec<SimpleExpr>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum SimpleExpr {
    Variable(var::Var),
    Indexing(Ind),
    Number(Num),
    String(String),
    Object(Obj),
    Array(Arr),
    Null,
    Bool(bool),
    Cases(NonEmptyVec<(SimpleExpr, SimpleExpr)>),
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
mod var;

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
    use once_cell::sync::OnceCell;
    use quickcheck::{Arbitrary, Gen, QuickCheck};
    use std::{cell::RefCell, convert::TryInto, str::FromStr};

    impl Query {
        pub fn new(from: TagExpr) -> Self {
            Self {
                features: vec![],
                from,
                ops: vec![],
            }
        }
        pub fn push(&mut self, op: Operation) {
            self.ops.push(op);
        }
        pub fn with_op(self, op: Operation) -> Self {
            let mut ops = self.ops;
            ops.push(op);
            Self { ops, ..self }
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
            Index::String((*self).to_owned())
        }
    }
    impl ToIndex for u64 {
        fn into(&self) -> Index {
            Index::Number(*self)
        }
    }

    thread_local! {
        static DEPTH: RefCell<usize> = RefCell::new(0);
    }

    macro_rules! arb {
        ($T:ident: $g:ident => $($n:ident)*, $($rec:ident)*, $($e:ident)*) => {{
            $(
                #[allow(non_snake_case)]
                fn $n(g: &mut Gen) -> $T {
                    $T::$n(Arbitrary::arbitrary(g))
                }
            )*
            $(
                #[allow(non_snake_case)]
                fn $rec(g: &mut Gen) -> $T {
                    $T::$rec(Arbitrary::arbitrary(g))
                }
            )*
            $(
                #[allow(non_snake_case)]
                fn $e(_g: &mut Gen) -> $T {
                    $T::$e
                }
            )*
            let choices = DEPTH.with(|depth| -> &[fn(&mut Gen) -> $T] {
                if *depth.borrow() > 4 {
                    &[$($n as fn(&mut Gen) -> $T,)* $($e,)*][..]
                } else {
                    &[$($n,)* $($rec,)* $($e,)*][..]
                }
            });
            let ret = ($g.choose(choices).unwrap())($g);
            match &ret {
                $($T::$n(_) => {})*
                $($T::$rec(_) => { DEPTH.with(|d| *d.borrow_mut() += 1) })*
                $($T::$e => {})*
            }
            ret
        }};
    }

    macro_rules! shrink {
        ($T:ident: $s:ident => $($n:ident)*, $($rec:ident)*,) => {
            match $s {
                $($T::$n(x) => Box::new(x.shrink().map($T::$n)),)*
                $($T::$rec(x) => Box::new(x.shrink().map($T::$rec)),)*
            }
        };
        ($T:ident: $s:ident => $($n:ident)*, $($rec:ident($m:ident,$($ex:expr),*))*, $($e:ident)*) => {
            match $s {
                $($T::$n(x) => Box::new(x.shrink().map($T::$n)),)*
                $($T::$rec($m) => Box::new(vec![$($ex,)*].into_iter().chain($m.shrink().map($T::$rec))),)*
                $($T::$e => quickcheck::empty_shrinker(),)*
            }
        };
    }

    impl Ind {
        pub fn with(head: impl Into<String>, tail: &[&dyn ToIndex]) -> SimpleExpr {
            SimpleExpr::Indexing(Self {
                head: Box::new(SimpleExpr::Variable(head.into().try_into().unwrap())),
                tail: tail.iter().map(|x| (*x).into()).collect::<Vec<_>>().try_into().unwrap(),
            })
        }
    }

    impl Obj {
        pub fn with(props: &[(&str, SimpleExpr)]) -> SimpleExpr {
            SimpleExpr::Object(Obj {
                props: props
                    .iter()
                    .map(|(x, e)| (Index::String((*x).to_owned()), e.clone()))
                    .collect(),
            })
        }
    }

    impl Arr {
        pub fn with(items: &[SimpleExpr]) -> SimpleExpr {
            SimpleExpr::Array(Arr { items: items.to_vec() })
        }
    }

    impl Arbitrary for Num {
        fn arbitrary(g: &mut Gen) -> Self {
            fn natural(g: &mut Gen) -> Num {
                Num::Natural(u64::arbitrary(g))
            }
            fn decimal(g: &mut Gen) -> Num {
                let mut n;
                loop {
                    n = f64::arbitrary(g);
                    if n.is_finite() {
                        break;
                    }
                }
                Num::Decimal(n)
            }
            let choices = &[natural, decimal][..];
            (g.choose(choices).unwrap())(g)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(Num: self => Natural Decimal,,)
        }
    }

    impl Arbitrary for Index {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(Index: g => String Number, Expr,)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            match self {
                Index::String(s) => Box::new(s.shrink().map(Index::String)),
                Index::Number(n) => Box::new(n.shrink().map(Index::Number)),
                Index::Expr(e) => Box::new(std::iter::once(Index::Number(0)).chain(e.shrink().map(Index::Expr))),
            }
        }
    }

    impl Arbitrary for Ind {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                head: Box::new(SimpleExpr::arbitrary(g)),
                tail: Arbitrary::arbitrary(g),
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let head = self.head.clone();
            let tail = self.tail.clone();
            Box::new(
                self.head
                    .shrink()
                    .map(move |head| Self {
                        head,
                        tail: tail.clone(),
                    })
                    .chain(self.tail.shrink().map(move |tail| Self {
                        head: head.clone(),
                        tail,
                    })),
            )
        }
    }

    impl Arbitrary for Arr {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                items: Arbitrary::arbitrary(g),
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.items.shrink().map(|items| Self { items }))
        }
    }

    impl Arbitrary for Obj {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                props: Arbitrary::arbitrary(g),
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.props.shrink().map(|props| Self { props }))
        }
    }

    impl Arbitrary for SimpleExpr {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(SimpleExpr: g => Variable Number String Bool, Indexing Object Array Cases Add Sub Mul Div Mod Pow And Or Not Xor Lt Le Gt Ge Eq Ne, Null)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(SimpleExpr: self => Variable Number String Bool,
                Indexing(x, (*x.head).clone())
                Object(x, x.props.first().map(|p| p.1.clone()).unwrap_or(SimpleExpr::Null))
                Array(x, x.items.first().cloned().unwrap_or(SimpleExpr::Null))
                Cases(x, x.first().map(|p| p.1.clone()).unwrap_or(SimpleExpr::Null))
                Add(x, x.0.clone(), x.1.clone())
                Sub(x, x.0.clone(), x.1.clone())
                Mul(x, x.0.clone(), x.1.clone())
                Div(x, x.0.clone(), x.1.clone())
                Mod(x, x.0.clone(), x.1.clone())
                Pow(x, x.0.clone(), x.1.clone())
                And(x, x.0.clone(), x.1.clone())
                Or(x, x.0.clone(), x.1.clone())
                Not(x, (**x).clone())
                Xor(x, x.0.clone(), x.1.clone())
                Lt(x, x.0.clone(), x.1.clone())
                Le(x, x.0.clone(), x.1.clone())
                Gt(x, x.0.clone(), x.1.clone())
                Ge(x, x.0.clone(), x.1.clone())
                Eq(x, x.0.clone(), x.1.clone())
                Ne(x, x.0.clone(), x.1.clone())
                , Null)
        }
    }

    impl Arbitrary for TagAtom {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TagAtom: g => Tag FromTime ToTime FromLamport ToLamport AppId, , AllEvents IsLocal)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TagAtom: self => Tag FromTime ToTime FromLamport ToLamport AppId, , AllEvents IsLocal)
        }
    }

    impl Arbitrary for TagExpr {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TagExpr: g => Atom, And Or,)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TagExpr: self => Atom, And(x, x.0.clone(), x.1.clone()) Or(x, x.0.clone(), x.1.clone()),)
        }
    }

    impl Arbitrary for Operation {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(Operation: g => Filter Select,,)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(Operation: self => Filter Select,,)
        }
    }

    impl Arbitrary for Query {
        fn arbitrary(g: &mut Gen) -> Self {
            fn word(g: &mut Gen) -> String {
                static CHOICES: OnceCell<Vec<char>> = OnceCell::new();
                let choices = CHOICES.get_or_init(|| ('a'..='z').chain('A'..='Z').chain('0'..='9').collect());
                let len = Vec::<()>::arbitrary(g).len().max(1);
                (0..len).map(|_| g.choose(choices).unwrap()).collect()
            }
            Self {
                features: Vec::<bool>::arbitrary(g).into_iter().map(|_| word(g)).collect(),
                from: TagExpr::arbitrary(g),
                ops: Arbitrary::arbitrary(g),
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let features = self.features.clone();
            let features2 = self.features.clone();
            let from = self.from.clone();
            let ops = self.ops.clone();
            Box::new(
                self.ops
                    .shrink()
                    .map(move |ops| Self {
                        features: features.clone(),
                        from: from.clone(),
                        ops,
                    })
                    .chain(self.from.shrink().map(move |from| Self {
                        features: features2.clone(),
                        from,
                        ops: ops.clone(),
                    })),
            )
        }
    }

    #[test]
    fn qc_roundtrip() {
        fn roundtrip_aql(q: Query) -> anyhow::Result<bool> {
            // What this test currently ascertains is that our rendered string is isomorphic
            // to the internal representation of the parse tree, hence there are many “unnecessary”
            // parentheses in the output. If we want to remove those parentheses, we need to
            // formulate a proper canonicalisation strategy and apply it to the source query as well
            // as during parsing. Luckily, this test will then prove that our canonicalisation
            // actually works.
            let s = q.to_string();
            let p = Query::from_str(&s)?;
            anyhow::ensure!(q == p, "q={} p={} pp={:?}", q, p, p);
            Ok(true)
        }
        let mut q = QuickCheck::new();
        if std::env::var_os("QUICKCHECK_TESTS").is_none() {
            q = q.tests(10_000);
        }
        q.max_tests(1_000_000)
            .gen(Gen::new(10))
            .quickcheck(roundtrip_aql as fn(Query) -> anyhow::Result<bool>)
    }
}
