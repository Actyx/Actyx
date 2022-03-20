mod non_empty;
mod parser;
mod render;

pub use self::non_empty::NonEmptyVec;
use self::render::render_tag_expr;
use crate::{service::Order, tags::Tag, AppId, EventKey, LamportTimestamp, StreamId, Timestamp};
use std::{convert::TryInto, fmt::Display, num::NonZeroU64, ops::Deref, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Events { from: TagExpr, order: Option<Order> },
    Array(Arr),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
/// A [`Query`] can be constructed using the Actyx Query Language (AQL). For an in-depth overview
/// see the [docs](https://developer.actyx.com/docs/reference/aql).
///
/// ```
/// use actyx_sdk::language::Query;
///
/// let query: Query = r#"
/// FEATURES(some features)  -- this is optional
/// FROM 'mytag1' & 'mytag2' -- the only mandatory part
/// SELECT _.value           -- optional list of transformations
/// END                      -- optional"#.parse().unwrap();
/// ```
pub struct Query {
    pub features: Vec<String>,
    pub source: Source,
    pub ops: Vec<Operation>,
}
mod query_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(NonEmptyVec<SpreadExpr>),
    Aggregate(SimpleExpr),
    Limit(NonZeroU64),
    Binding(String, SimpleExpr),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpreadExpr {
    pub expr: SimpleExpr,
    pub spread: bool,
}
impl Deref for SpreadExpr {
    type Target = SimpleExpr;
    fn deref(&self) -> &Self::Target {
        &self.expr
    }
}
impl Display for SpreadExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.spread {
            write!(f, "...{}", self.expr)
        } else {
            self.expr.fmt(f)
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagExpr {
    Or(Arc<(TagExpr, TagExpr)>),
    And(Arc<(TagExpr, TagExpr)>),
    Atom(TagAtom),
}

impl TagExpr {
    pub fn and(self, other: TagExpr) -> Self {
        TagExpr::And(Arc::new((self, other)))
    }
    pub fn or(self, other: TagExpr) -> Self {
        TagExpr::Or(Arc::new((self, other)))
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct SortKey {
    pub lamport: LamportTimestamp,
    pub stream: StreamId,
}

impl SortKey {
    pub fn new(lamport: LamportTimestamp, stream: StreamId) -> Self {
        Self { lamport, stream }
    }
}

impl From<EventKey> for SortKey {
    fn from(k: EventKey) -> Self {
        Self {
            lamport: k.lamport,
            stream: k.stream,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagAtom {
    Tag(Tag),
    Interpolation(Vec<SimpleExpr>),
    AllEvents,
    IsLocal,
    FromTime(Timestamp),
    ToTime(Timestamp),
    FromLamport(SortKey),
    ToLamport(SortKey),
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

pub use parser::is_ident;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Ind {
    pub head: Arc<SimpleExpr>,
    pub tail: NonEmptyVec<Index>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct FuncCall {
    pub name: String,
    pub args: Arc<[SimpleExpr]>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Obj {
    pub props: Arc<[(Index, SimpleExpr)]>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Arr {
    pub items: Arc<[SpreadExpr]>,
}

macro_rules! decl_op {
    ($(#[$a:meta])* $v:vis enum $n:ident { $($x:ident -> $s:literal,)* }) => {
        $(#[$a])* $v enum $n {
            $($x,)*
        }

        impl $n {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($n::$x => $s,)*
                }
            }
        }

        #[cfg(test)]
        impl ::quickcheck::Arbitrary for $n {
            fn arbitrary(g: &mut ::quickcheck::Gen) -> Self {
                *g.choose(&[$($n::$x,)*]).unwrap()
            }
        }
    }
}

decl_op! {
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub enum BinOp {
        Add -> "+",
        Sub -> "-",
        Mul -> "*",
        Div -> "/",
        Mod -> "%",
        Pow -> "^",
        And -> "&",
        Or -> "|",
        Xor -> "~",
        Lt -> "<",
        Le -> "<=",
        Gt -> ">",
        Ge -> ">=",
        Eq -> "=",
        Ne -> "!=",
        Alt -> "??",
    }
}

decl_op! {
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub enum AggrOp {
        Sum -> "SUM",
        Prod -> "PRODUCT",
        Min -> "MIN",
        Max -> "MAX",
        First -> "FIRST",
        Last -> "LAST",
    }
}

mod var;
pub use var::Var;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum SimpleExpr {
    Variable(Var),
    Indexing(Ind),
    Number(Num),
    String(String),
    Interpolation(Vec<SimpleExpr>),
    Object(Obj),
    Array(Arr),
    Null,
    Bool(bool),
    Cases(NonEmptyVec<(SimpleExpr, SimpleExpr)>),
    BinOp(Arc<(BinOp, SimpleExpr, SimpleExpr)>),
    Not(Arc<SimpleExpr>),
    AggrOp(Arc<(AggrOp, SimpleExpr)>),
    FuncCall(FuncCall),
    SubQuery(Query),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traverse {
    Descend,
    Stop,
}

impl SimpleExpr {
    pub fn with_spread(self, spread: bool) -> SpreadExpr {
        SpreadExpr { expr: self, spread }
    }

    /// Traverse all parts of the expression, including expressions in sub-queries
    pub fn traverse(&self, f: &mut impl FnMut(&SimpleExpr) -> Traverse) {
        if f(self) == Traverse::Descend {
            match self {
                SimpleExpr::Variable(_) => {}
                SimpleExpr::Indexing(Ind { head, tail }) => {
                    head.traverse(f);
                    for t in tail.iter() {
                        match t {
                            Index::String(_) => {}
                            Index::Number(_) => {}
                            Index::Expr(e) => e.traverse(f),
                        }
                    }
                }
                SimpleExpr::Number(_) => {}
                SimpleExpr::String(_) => {}
                SimpleExpr::Interpolation(e) => {
                    for expr in e.iter() {
                        expr.traverse(f);
                    }
                }
                SimpleExpr::Object(Obj { props }) => {
                    for (idx, expr) in props.iter() {
                        match idx {
                            Index::String(_) => {}
                            Index::Number(_) => {}
                            Index::Expr(e) => e.traverse(f),
                        }
                        expr.traverse(f);
                    }
                }
                SimpleExpr::Array(Arr { items }) => {
                    for expr in items.iter() {
                        expr.traverse(f);
                    }
                }
                SimpleExpr::Null => {}
                SimpleExpr::Bool(_) => {}
                SimpleExpr::Cases(c) => {
                    for (cond, expr) in c.iter() {
                        cond.traverse(f);
                        expr.traverse(f);
                    }
                }
                SimpleExpr::BinOp(x) => {
                    x.1.traverse(f);
                    x.2.traverse(f);
                }
                SimpleExpr::Not(e) => e.traverse(f),
                SimpleExpr::AggrOp(a) => a.1.traverse(f),
                SimpleExpr::FuncCall(c) => {
                    for expr in c.args.iter() {
                        expr.traverse(f);
                    }
                }
                SimpleExpr::SubQuery(q) => {
                    match &q.source {
                        Source::Events { .. } => {}
                        Source::Array(Arr { items }) => {
                            for e in items.iter() {
                                e.traverse(f);
                            }
                        }
                    }
                    for op in q.ops.iter() {
                        match op {
                            Operation::Filter(e) => e.traverse(f),
                            Operation::Select(e) => {
                                for expr in e.iter() {
                                    expr.traverse(f);
                                }
                            }
                            Operation::Aggregate(e) => e.traverse(f),
                            Operation::Limit(_) => {}
                            Operation::Binding(_, e) => e.traverse(f),
                        }
                    }
                }
            }
        }
    }

    /// Rewrite parts of the expression tree where the provided function returns Some
    ///
    /// Also traverses into the expressions contained in sub-queries.
    pub fn rewrite(&self, f: &mut impl FnMut(&SimpleExpr) -> Option<SimpleExpr>) -> Self {
        if let Some(expr) = f(self) {
            return expr;
        }
        match self {
            SimpleExpr::Variable(_v) => self.clone(),
            SimpleExpr::Indexing(Ind { head, tail }) => {
                let head = Arc::new(head.rewrite(f));
                let tail = tail
                    .iter()
                    .map(|i| match i {
                        Index::String(_) => i.clone(),
                        Index::Number(_) => i.clone(),
                        Index::Expr(e) => Index::Expr(e.rewrite(f)),
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                SimpleExpr::Indexing(Ind { head, tail })
            }
            SimpleExpr::Number(_n) => self.clone(),
            SimpleExpr::String(_s) => self.clone(),
            SimpleExpr::Interpolation(s) => {
                let s = s.iter().map(|e| e.rewrite(f)).collect();
                SimpleExpr::Interpolation(s)
            }
            SimpleExpr::Object(Obj { props }) => {
                let props = props
                    .iter()
                    .map(|(i, e)| {
                        let i = match i {
                            Index::String(_s) => i.clone(),
                            Index::Number(_n) => i.clone(),
                            Index::Expr(e) => Index::Expr(e.rewrite(f)),
                        };
                        let e = e.rewrite(f);
                        (i, e)
                    })
                    .collect();
                SimpleExpr::Object(Obj { props })
            }
            SimpleExpr::Array(Arr { items }) => {
                let items = items
                    .iter()
                    .map(|e| SpreadExpr {
                        expr: e.rewrite(f),
                        spread: e.spread,
                    })
                    .collect::<Vec<_>>();
                SimpleExpr::Array(Arr { items: items.into() })
            }
            SimpleExpr::Null => self.clone(),
            SimpleExpr::Bool(_) => self.clone(),
            SimpleExpr::Cases(c) => {
                let c = c
                    .iter()
                    .map(|(cond, expr)| (cond.rewrite(f), expr.rewrite(f)))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                SimpleExpr::Cases(c)
            }
            SimpleExpr::BinOp(o) => SimpleExpr::BinOp(Arc::new((o.0, o.1.rewrite(f), o.2.rewrite(f)))),
            SimpleExpr::Not(e) => SimpleExpr::Not(Arc::new(e.rewrite(f))),
            SimpleExpr::AggrOp(a) => SimpleExpr::AggrOp(Arc::new((a.0, a.1.rewrite(f)))),
            SimpleExpr::FuncCall(FuncCall { name, args }) => {
                let args = args.iter().map(|e| e.rewrite(f)).collect();
                SimpleExpr::FuncCall(FuncCall {
                    name: name.clone(),
                    args,
                })
            }
            SimpleExpr::SubQuery(q) => {
                let features = q.features.clone();
                let source = match &q.source {
                    Source::Events { from, order } => Source::Events {
                        from: from.clone(),
                        order: *order,
                    },
                    Source::Array(Arr { items }) => Source::Array(Arr {
                        items: items
                            .iter()
                            .map(|e| SpreadExpr {
                                expr: e.rewrite(f),
                                spread: e.spread,
                            })
                            .collect::<Vec<_>>()
                            .into(),
                    }),
                };
                let ops = q
                    .ops
                    .iter()
                    .map(|op| match op {
                        Operation::Filter(e) => Operation::Filter(e.rewrite(f)),
                        Operation::Select(e) => Operation::Select(
                            e.iter()
                                .map(|e| SpreadExpr {
                                    expr: e.rewrite(f),
                                    spread: e.spread,
                                })
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap(),
                        ),
                        Operation::Aggregate(a) => Operation::Aggregate(a.rewrite(f)),
                        Operation::Limit(l) => Operation::Limit(*l),
                        Operation::Binding(n, e) => Operation::Binding(n.clone(), e.rewrite(f)),
                    })
                    .collect();
                SimpleExpr::SubQuery(Query { features, source, ops })
            }
        }
    }
}

impl std::fmt::Display for SimpleExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render::render_simple_expr(f, self)
    }
}

#[allow(clippy::should_implement_trait)]
impl SimpleExpr {
    pub fn add(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Add, self, other)))
    }
    pub fn sub(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Sub, self, other)))
    }
    pub fn mul(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Mul, self, other)))
    }
    pub fn div(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Div, self, other)))
    }
    pub fn modulo(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Mod, self, other)))
    }
    pub fn pow(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Pow, self, other)))
    }
    pub fn and(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::And, self, other)))
    }
    pub fn or(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Or, self, other)))
    }
    pub fn xor(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Xor, self, other)))
    }
    pub fn lt(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Lt, self, other)))
    }
    pub fn le(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Le, self, other)))
    }
    pub fn gt(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Gt, self, other)))
    }
    pub fn ge(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Ge, self, other)))
    }
    pub fn eq(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Eq, self, other)))
    }
    pub fn ne(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Ne, self, other)))
    }
    pub fn alt(self, other: SimpleExpr) -> Self {
        SimpleExpr::BinOp(Arc::new((BinOp::Alt, self, other)))
    }
}

#[cfg(test)]
mod for_tests {
    use super::{parser::Context, *};
    use once_cell::sync::OnceCell;
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::{cell::RefCell, convert::TryInto, str::FromStr};

    impl Query {
        pub fn new(from: TagExpr) -> Self {
            Self {
                features: vec![],
                source: Source::Events { from, order: None },
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
            TagExpr::And(Arc::new((TagExpr::Atom(self), TagExpr::Atom(other))))
        }
        pub fn or(self, other: TagAtom) -> TagExpr {
            TagExpr::Or(Arc::new((TagExpr::Atom(self), TagExpr::Atom(other))))
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
        static CTX: RefCell<Context> = RefCell::new(Context::Simple);
    }

    macro_rules! arb {
        ($T:ident: $g:ident => $($n:ident$({$extra:expr})?)*, $($rec:ident)*, $($rec2:ident$({$extra2:expr})?)*, $($e:ident)* $(, $($names:ident)+)?) => {{
            $(
                #[allow(non_snake_case)]
                fn $n(g: &mut Gen) -> $T {
                    $(let prev = CTX.with(|c| c.replace($extra));)*
                    let ret = $T::$n(Arbitrary::arbitrary(g));
                    $(CTX.with(|c| c.replace(prev)); stringify!($extra);)*
                    ret
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
                fn $rec2(g: &mut Gen) -> $T {
                    $(let prev = CTX.with(|c| c.replace($extra2));)*
                    let ret = $T::$rec2(Arbitrary::arbitrary(g));
                    $(CTX.with(|c| c.replace(prev)); stringify!($extra2);)*
                    ret
                }
            )*
            $(
                #[allow(non_snake_case)]
                fn $e(_g: &mut Gen) -> $T {
                    $T::$e
                }
            )*
            let depth = DEPTH.with(|d| *d.borrow());
            let ctx = CTX.with(|c| *c.borrow());
            let choices = if depth > 5 {
                &[$($n as fn(&mut Gen) -> $T,)* $($e,)* $($($names,)*)?][..]
            } else if ctx == Context::Aggregate {
                &[$($n,)* $($rec,)* $($rec2,)* $($e,)* $($($names,)*)?][..]
            } else {
                &[$($n,)* $($rec,)* $($e,)* $($($names,)*)?][..]
            };
            DEPTH.with(|d| *d.borrow_mut() += 1);
            let ret = ($g.choose(choices).unwrap())($g);
            DEPTH.with(|d| *d.borrow_mut() -= 1);
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
        ($T:ident: $s:ident => $($n:ident)*, $($rec:ident($m:ident,$($ex:expr),*))*, $($e:ident)* $(, $pat:pat => $patex:expr )*) => {
            match $s {
                $($T::$n(x) => Box::new(x.shrink().map($T::$n)),)*
                $($T::$rec($m) => Box::new(vec![$($ex,)*].into_iter().chain($m.shrink().map($T::$rec))),)*
                $($T::$e => quickcheck::empty_shrinker(),)*
                $($pat => $patex,)*
            }
        };
    }

    impl Ind {
        pub fn with(head: impl Into<String>, tail: &[&dyn ToIndex]) -> SimpleExpr {
            SimpleExpr::Indexing(Self {
                head: Arc::new(SimpleExpr::Variable(head.into().try_into().unwrap())),
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
            SimpleExpr::Array(Arr {
                items: items
                    .iter()
                    .map(|expr| SpreadExpr {
                        expr: expr.clone(),
                        spread: false,
                    })
                    .collect::<Vec<_>>()
                    .into(),
            })
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
            arb!(Index: g => String Number, Expr,,)
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
                head: Arc::new(SimpleExpr::arbitrary(g)),
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

    impl Arbitrary for FuncCall {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                name: "Ab".to_owned(),
                args: Vec::arbitrary(g).into(),
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let name = self.name.clone();
            Box::new(self.args.to_vec().shrink().map(move |args| Self {
                name: name.clone(),
                args: args.into(),
            }))
        }
    }

    impl Arbitrary for Arr {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                items: Vec::arbitrary(g).into(),
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.items.to_vec().shrink().map(|items| Self { items: items.into() }))
        }
    }

    impl Arbitrary for Obj {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                props: Vec::arbitrary(g).into(),
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.props.to_vec().shrink().map(|props| Self { props: props.into() }))
        }
    }

    impl Arbitrary for SimpleExpr {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(SimpleExpr: g =>
                Variable Number String Bool,
                Indexing Object Array Cases BinOp Not FuncCall SubQuery Interpolation,
                AggrOp{ Context::Simple },
                Null
            )
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(SimpleExpr: self => Variable Number String Bool,
                Indexing(x, (*x.head).clone())
                Interpolation(x,)
                Object(x, x.props.first().map(|p| p.1.clone()).unwrap_or(SimpleExpr::Null))
                Array(x, x.items.first().map(|i| i.expr.clone()).unwrap_or(SimpleExpr::Null))
                Cases(x, x.first().map(|p| p.1.clone()).unwrap_or(SimpleExpr::Null))
                BinOp(x,)
                Not(x, (**x).clone())
                AggrOp(x,)
                FuncCall(x,)
                SubQuery(x,)
                , Null)
        }
    }

    impl Arbitrary for SpreadExpr {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                expr: SimpleExpr::arbitrary(g),
                spread: bool::arbitrary(g),
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let spread = self.spread;
            Box::new(self.expr.shrink().map(move |expr| SpreadExpr { expr, spread }))
        }
    }

    impl Arbitrary for TagAtom {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TagAtom: g => Tag FromTime ToTime FromLamport ToLamport AppId, Interpolation, , AllEvents IsLocal)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TagAtom: self => Tag FromTime ToTime FromLamport ToLamport AppId, Interpolation(x,), AllEvents IsLocal)
        }
    }

    impl Arbitrary for TagExpr {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TagExpr: g => Atom, And Or,,)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TagExpr: self => Atom, And(x, x.0.clone(), x.1.clone()) Or(x, x.0.clone(), x.1.clone()),)
        }
    }

    impl Arbitrary for Operation {
        fn arbitrary(g: &mut Gen) -> Self {
            #[allow(non_snake_case)]
            fn Binding(g: &mut Gen) -> Operation {
                Operation::Binding(Var::arbitrary(g).0, SimpleExpr::arbitrary(g))
            }
            arb!(Operation: g => Filter Select Aggregate{ Context::Aggregate } Limit,,,, Binding)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(Operation: self => Filter Select Aggregate Limit,,,
                Operation::Binding(n, e) => {
                    let n = n.clone();
                    Box::new(e.shrink().map(move |e| Operation::Binding(n.clone(), e)))
                }
            )
        }
    }

    impl Arbitrary for Order {
        fn arbitrary(g: &mut Gen) -> Self {
            *g.choose(&[Order::Asc, Order::Desc, Order::StreamAsc]).unwrap()
        }
    }

    impl Arbitrary for Source {
        fn arbitrary(g: &mut Gen) -> Self {
            if bool::arbitrary(g) {
                Self::Events {
                    from: TagExpr::arbitrary(g),
                    order: Arbitrary::arbitrary(g),
                }
            } else {
                Self::Array(Arr::arbitrary(g))
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            match self {
                Source::Events { from, order } => {
                    let order = *order;
                    Box::new(from.shrink().map(move |from| Source::Events { from, order }))
                }
                Source::Array(arr) => Box::new(arr.shrink().map(Source::Array)),
            }
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
            let prev = CTX.with(|c| c.replace(Context::Simple));
            let source = Source::arbitrary(g);
            let ret = Self {
                features: Vec::<bool>::arbitrary(g).into_iter().map(|_| word(g)).collect(),
                source,
                ops: Arbitrary::arbitrary(g),
            };
            CTX.with(|c| c.replace(prev));
            ret
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let features = self.features.clone();
            let features2 = self.features.clone();
            let source = self.source.clone();
            let ops = self.ops.clone();
            Box::new(
                self.ops
                    .shrink()
                    .map(move |ops| Self {
                        features: features.clone(),
                        source: source.clone(),
                        ops,
                    })
                    .chain(self.source.shrink().map(move |source| Self {
                        features: features2.clone(),
                        source,
                        ops: ops.clone(),
                    })),
            )
        }
    }

    #[test]
    fn qc_roundtrip() {
        fn roundtrip_aql(q: Query) -> TestResult {
            // What this test currently ascertains is that our rendered string is isomorphic
            // to the internal representation of the parse tree, hence there are many “unnecessary”
            // parentheses in the output. If we want to remove those parentheses, we need to
            // formulate a proper canonicalisation strategy and apply it to the source query as well
            // as during parsing. Luckily, this test will then prove that our canonicalisation
            // actually works.
            let s = q.to_string();
            let p = match Query::from_str(&s) {
                Ok(p) => p,
                Err(e) => return TestResult::error(e.to_string()),
            };
            if q == p {
                TestResult::passed()
            } else {
                TestResult::error(format!("\nq={}\np={}\n ={:?}", q, p, p))
            }
        }
        let mut q = QuickCheck::new();
        if std::env::var_os("QUICKCHECK_TESTS").is_none() {
            q = q.tests(200);
        }
        q.max_tests(1_000_000)
            .gen(Gen::new(10))
            .quickcheck(roundtrip_aql as fn(Query) -> TestResult)
    }
}
