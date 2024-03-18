macro_rules! unexpected {
    ($label:ident) => {
        return Err(anyhow::Error::from(pest::error::Error::new_from_span(
            pest::error::ErrorVariant::<Rule>::CustomError {
                message: format!("unexpected token ({}: {}): {:?}", file!(), line!(), $label.as_rule()),
            },
            $label.as_span(),
        )))
    };
}

mod non_empty;
mod parse_utils;
mod parser;
mod render;
mod type_check;
mod types;
mod workflow;

pub use self::{
    non_empty::NonEmptyVec,
    rewrite_impl::{Galactus, Tactic},
    types::{Label, Type, TypeAtom},
};

use self::{non_empty::NonEmptyString, parser::SingleTag, render::render_tag_expr};
use ax_types::{service::Order, AppId, EventKey, LamportTimestamp, StreamId, Tag, Timestamp};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    num::NonZeroU64,
    ops::Deref,
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Events { from: TagExpr, order: Option<Order> },
    Array(Arr<SpreadExpr>),
}

pub struct StaticQuery(pub Query<'static>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
/// A [`Query`] can be constructed using the Actyx Query Language (AQL). For an in-depth overview
/// see the [docs](https://developer.actyx.com/docs/reference/aql).
///
/// ```
/// use ax_aql::Query;
///
/// let query = Query::parse(r#"
/// FROM 'mytag1' & 'mytag2' -- the only mandatory part
/// SELECT _.value           -- optional list of transformations
/// END                      -- optional"#).unwrap();
/// ```
pub struct Query<'a> {
    pub pragmas: Vec<(&'a str, &'a str)>,
    pub features: Vec<String>,
    pub source: Source,
    pub ops: Vec<Operation>,
    pub events: Arc<BTreeMap<Ident, (Type, Vec<SingleTag>)>>,
    pub workflows: Arc<BTreeMap<Ident, workflow::Workflow<'a>>>,
}

mod query_impl;
mod rewrite_impl;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Ident(NonEmptyString);

impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(NonEmptyVec<SpreadExpr>),
    Aggregate(SimpleExpr),
    Limit(NonZeroU64),
    Binding(String, SimpleExpr),
    Machine(Ident, Ident, NonEmptyString),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl fmt::Display for TagExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl Display for SortKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", u64::from(self.lamport), self.stream)
    }
}

impl SortKey {
    pub fn new(lamport: LamportTimestamp, stream: StreamId) -> Self {
        Self { lamport, stream }
    }

    pub fn succ(self) -> Self {
        let Self { lamport, stream } = self;
        let stream = stream.node_id().stream(stream.stream_nr().succ());
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
    Interpolation(Arr<SimpleExpr>),
    AllEvents,
    IsLocal,
    FromTime(Timestamp, bool),
    ToTime(Timestamp, bool),
    FromLamport(SortKey, bool),
    ToLamport(SortKey, bool),
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
pub struct Arr<T> {
    pub items: Arc<[T]>,
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
    Interpolation(Arr<SimpleExpr>),
    Object(Obj),
    Array(Arr<SpreadExpr>),
    Null,
    Bool(bool),
    Cases(NonEmptyVec<(SimpleExpr, SimpleExpr)>),
    BinOp(Arc<(BinOp, SimpleExpr, SimpleExpr)>),
    Not(Arc<SimpleExpr>),
    AggrOp(Arc<(AggrOp, SimpleExpr)>),
    FuncCall(FuncCall),
    SubQuery(Query<'static>),
    KeyVar(Var),
    KeyLiteral(SortKey),
    TimeVar(Var),
    TimeLiteral(Timestamp),
    Tags(Var),
    App(Var),
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
                    for expr in e.items.iter() {
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
                            Operation::Machine(_, _, _) => {}
                        }
                    }
                }
                SimpleExpr::KeyVar(_) => {}
                SimpleExpr::KeyLiteral(_) => {}
                SimpleExpr::TimeVar(_) => {}
                SimpleExpr::TimeLiteral(_) => {}
                SimpleExpr::Tags(_) => {}
                SimpleExpr::App(_) => {}
            }
        }
    }
}

impl fmt::Display for SimpleExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    use self::{
        parse_utils::Span,
        workflow::{Binding, EventMode, Participant, Workflow, WorkflowStep},
    };
    use super::{parser::Context, *};
    use once_cell::sync::OnceCell;
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::{cell::RefCell, convert::TryInto, mem::replace, ops::Range};

    impl<'a> Query<'a> {
        pub fn new(from: TagExpr) -> Self {
            Self {
                pragmas: Vec::new(),
                features: vec![],
                source: Source::Events { from, order: None },
                ops: vec![],
                events: Arc::new(BTreeMap::new()),
                workflows: Arc::new(BTreeMap::new()),
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
        static CTX: RefCell<Context> = RefCell::new(Context::Simple { now: Timestamp::now() });
        static SUB: RefCell<bool> = RefCell::new(true);
    }

    macro_rules! arb {
        ($T:ident: $g:ident => $($n:ident$(($t:ty))*$({$extra:expr})?)*, $($rec:ident)*, $($rec2:ident$({$extra2:expr})?)*, $($e:ident)* $(, $($names:ident)+)?) => {{
            $(
                #[allow(non_snake_case)]
                fn $n(g: &mut Gen) -> $T {
                    $(let prev = CTX.with(|c| c.replace($extra));)*
                    let ret = $T::$n(Arbitrary::arbitrary(g) $(,<$t>::arbitrary(g))*);
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
            let choices = if depth > 4 {
                &[$($n as fn(&mut Gen) -> $T,)* $($e,)* $($($names,)*)?][..]
            } else if ctx.is_aggregate() {
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
        ($T:ident: $s:ident => $($n:ident$(($add:ident))*)*, $($rec:ident($m:ident,$($ex:expr),*))*, $($e:ident)* $(, $pat:pat => $patex:expr )*) => {
            match $s {
                $($T::$n(x $(,$add)*) => {
                    $(let $add = *$add;)*
                    Box::new(x.shrink().map(move |x| $T::$n(x $(,$add.clone())*)))
                })*
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

    impl Arr<SpreadExpr> {
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

    impl Arbitrary for Label {
        fn arbitrary(g: &mut Gen) -> Self {
            #[allow(non_snake_case)]
            fn String(g: &mut Gen) -> Label {
                Label::String(Arbitrary::arbitrary(g))
            }
            #[allow(non_snake_case)]
            fn Number(g: &mut Gen) -> Label {
                Label::Number(Arbitrary::arbitrary(g))
            }
            let choices = &[String, Number][..];
            (g.choose(choices).unwrap())(g)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            match self {
                Label::String(s) => Box::new(s.shrink().map(Label::String)),
                Label::Number(n) => Box::new(n.shrink().map(Label::Number)),
            }
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

    impl<T: Arbitrary> Arbitrary for Arr<T> {
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
            #[allow(non_snake_case)]
            fn SubQuery(g: &mut Gen) -> SimpleExpr {
                let mut query = Query::arbitrary(g);
                query.features.clear();
                SimpleExpr::SubQuery(query)
            }
            if SUB.with(|s| *s.borrow()) {
                arb!(SimpleExpr: g =>
                    Variable Number String Bool KeyLiteral KeyVar TimeLiteral TimeVar Tags App,
                    Indexing Object Array Cases BinOp Not FuncCall Interpolation,
                    AggrOp{ Context::Simple { now: Timestamp::now() } },
                    Null,
                    SubQuery
                )
            } else {
                arb!(SimpleExpr: g =>
                    Variable Number String Bool KeyLiteral KeyVar TimeLiteral TimeVar Tags App,
                    Indexing Object Array Cases BinOp Not FuncCall Interpolation,
                    AggrOp{ Context::Simple { now: Timestamp::now() } },
                    Null
                )
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(SimpleExpr: self => Variable Number String Bool KeyLiteral KeyVar TimeLiteral TimeVar Tags App,
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

    impl Arbitrary for SingleTag {
        fn arbitrary(g: &mut Gen) -> Self {
            let sub = SUB.with(|s| replace(&mut *s.borrow_mut(), false));
            let ret = arb!(SingleTag: g => Tag, Interpolation, ,);
            SUB.with(|s| *s.borrow_mut() = sub);
            ret
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(SingleTag: self => Tag, Interpolation(x,),)
        }
    }

    impl Arbitrary for TagAtom {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TagAtom: g => Tag FromTime(bool) ToTime(bool) FromLamport(bool) ToLamport(bool) AppId, Interpolation, , AllEvents IsLocal)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TagAtom: self => Tag FromTime(i) ToTime(i) FromLamport(i) ToLamport(i) AppId, Interpolation(x,), AllEvents IsLocal)
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

    impl Arbitrary for TypeAtom {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(TypeAtom: g => Bool Number String, Tuple Record,, Null Timestamp Universal)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(TypeAtom: self => Bool Number String, Tuple(x,) Record(x,), Null Timestamp Universal)
        }
    }

    impl Arbitrary for Type {
        fn arbitrary(g: &mut Gen) -> Self {
            arb!(Type: g => Atom, Union Intersection Array Dict,,)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(Type: self => Atom, Union(x, x.0.clone(), x.1.clone()) Intersection(x, x.0.clone(), x.1.clone()) Array(x,(**x).clone()) Dict(x,(**x).clone()), NoValue)
        }
    }

    impl Arbitrary for Operation {
        fn arbitrary(g: &mut Gen) -> Self {
            #[allow(non_snake_case)]
            fn Binding(g: &mut Gen) -> Operation {
                Operation::Binding(Var::arbitrary(g).0, SimpleExpr::arbitrary(g))
            }
            arb!(Operation: g => Filter Select Aggregate{ Context::Aggregate { now: Timestamp::now() } } Limit Machine(Ident)(NonEmptyString),,,, Binding)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            shrink!(Operation: self => Filter Select Aggregate Limit,,,
                Operation::Machine(..) => quickcheck::empty_shrinker(),
                Operation::Binding(n, e) => {
                    let n = n.clone();
                    Box::new(e.shrink().map(move |e| Operation::Binding(n.clone(), e)))
                }
            )
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

    impl Arbitrary for Participant {
        fn arbitrary(g: &mut Gen) -> Self {
            (g.choose(&[Participant::Role, Participant::Unique][..]).unwrap())(Ident::arbitrary(g))
        }
    }

    impl<T: Arbitrary> Arbitrary for Span<'static, T> {
        fn arbitrary(g: &mut Gen) -> Self {
            let span = pest::Span::new("", 0, 0).unwrap();
            Self::new(span, T::arbitrary(g))
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let span = self.span();
            Box::new(self.deref().shrink().map(move |x| Self::new(span, x)))
        }
    }

    impl Arbitrary for EventMode {
        fn arbitrary(g: &mut Gen) -> Self {
            *g.choose(&[EventMode::Fail, EventMode::Return, EventMode::Normal][..])
                .unwrap()
        }
    }

    impl Arbitrary for Binding {
        fn arbitrary(g: &mut Gen) -> Self {
            let sub = SUB.with(|s| replace(&mut *s.borrow_mut(), false));
            let value = SimpleExpr::arbitrary(g);
            SUB.with(|s| *s.borrow_mut() = sub);
            Self {
                name: Arbitrary::arbitrary(g),
                role: Arbitrary::arbitrary(g),
                value,
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let name = self.name.clone();
            let role = self.role.clone();
            Box::new(self.value.shrink().map(move |value| Self {
                name: name.clone(),
                role: role.clone(),
                value,
            }))
        }
    }

    fn ident_chars() -> &'static [char] {
        static CHOICES: OnceCell<Vec<char>> = OnceCell::new();
        &CHOICES.get_or_init(|| ('a'..='z').chain('A'..='Z').chain('0'..='9').collect())
    }
    const MINUSCULE: Range<usize> = 0..26;
    const MAJUSCULE: Range<usize> = 26..52;

    impl Arbitrary for Ident {
        fn arbitrary(g: &mut Gen) -> Self {
            fn minuscule(g: &mut Gen) -> NonEmptyString {
                let mut s = NonEmptyString::new(*g.choose(&ident_chars()[MINUSCULE]).unwrap());
                for _ in 0..g.size() {
                    s.push(*g.choose(&ident_chars()[..]).unwrap());
                }
                s
            }
            fn majuscule(g: &mut Gen) -> NonEmptyString {
                let mut s = NonEmptyString::new(*g.choose(&ident_chars()[MAJUSCULE]).unwrap());
                s.push(*g.choose(&ident_chars()[MINUSCULE]).unwrap());
                for _ in 0..g.size() {
                    s.push(*g.choose(&ident_chars()[..]).unwrap());
                }
                s
            }
            Self((g.choose(&[minuscule, majuscule][..]).unwrap())(g))
        }
    }

    macro_rules! shrink_struct {
        ($s:ident; $($n:ident),+; $m:ident) => {
            $s.$m.shrink().map({
                $(let $n = $s.$n.clone();)*
                move |$m| Self { $($n: $n.clone(),)* $m }
            })
        };
        ($($n:ident),+ - $m:ident; $v:ident) => {
            $m.shrink().map({
                $(let $n = $n.clone();)*
                move |$m| Self::$v { $($n: $n.clone(),)* $m }
            })
        };
    }

    impl Arbitrary for WorkflowStep<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            fn event(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Event {
                    state: Arbitrary::arbitrary(g),
                    mode: Arbitrary::arbitrary(g),
                    label: Arbitrary::arbitrary(g),
                    participant: Arbitrary::arbitrary(g),
                    binders: Arbitrary::arbitrary(g),
                }
            }
            fn retry(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Retry {
                    steps: Arbitrary::arbitrary(g),
                }
            }
            fn timeout(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Timeout {
                    micros: Arbitrary::arbitrary(g),
                    steps: Arbitrary::arbitrary(g),
                    mode: Arbitrary::arbitrary(g),
                    label: Arbitrary::arbitrary(g),
                    participant: Arbitrary::arbitrary(g),
                    binders: Arbitrary::arbitrary(g),
                }
            }
            fn parallel(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Parallel {
                    count: Arbitrary::arbitrary(g),
                    cases: Arbitrary::arbitrary(g),
                }
            }
            fn call(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Call {
                    workflow: Arbitrary::arbitrary(g),
                    args: Arbitrary::arbitrary(g),
                    cases: Arbitrary::arbitrary(g),
                }
            }
            fn compensate(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Compensate {
                    body: Arbitrary::arbitrary(g),
                    with: Arbitrary::arbitrary(g),
                }
            }
            fn choice(g: &mut Gen) -> WorkflowStep<'static> {
                WorkflowStep::Choice {
                    cases: Arbitrary::arbitrary(g),
                }
            }
            if DEPTH.with(|d| *d.borrow()) < 2 {
                DEPTH.with(|d| *d.borrow_mut() += 1);
                let choices = &[event, retry, timeout, parallel, call, compensate, choice][..];
                let ret = (g.choose(choices).unwrap())(g);
                DEPTH.with(|d| *d.borrow_mut() -= 1);
                ret
            } else {
                event(g)
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            match self {
                WorkflowStep::Event {
                    state,
                    mode,
                    label,
                    participant,
                    binders,
                } => {
                    let state = state.clone();
                    let mode = *mode;
                    let label = label.clone();
                    let participant = participant.clone();
                    let binders = binders.clone();
                    Box::new(binders.shrink().map(move |binders| Self::Event {
                        state: state.clone(),
                        mode,
                        label: label.clone(),
                        participant: participant.clone(),
                        binders,
                    }))
                }
                WorkflowStep::Retry { steps } => Box::new(steps.shrink().map(|steps| Self::Retry { steps })),
                WorkflowStep::Timeout {
                    micros,
                    steps,
                    mode,
                    label,
                    participant,
                    binders,
                } => {
                    let step = shrink_struct!(micros, mode, label, participant, binders - steps; Timeout);
                    let binder = shrink_struct!(micros, mode, label, participant, steps - binders; Timeout);
                    Box::new(step.chain(binder))
                }
                WorkflowStep::Parallel { count, cases } => {
                    let count = *count;
                    let cases = cases.clone();
                    Box::new(cases.shrink().map(move |cases| Self::Parallel { count, cases }))
                }
                WorkflowStep::Call { workflow, args, cases } => {
                    let arg = shrink_struct!(workflow, cases - args; Call);
                    let case = shrink_struct!(workflow, args - cases; Call);
                    Box::new(arg.chain(case))
                }
                WorkflowStep::Compensate { body, with } => {
                    let bod = shrink_struct!(with - body; Compensate);
                    let wit = shrink_struct!(body - with; Compensate);
                    Box::new(bod.chain(wit))
                }
                WorkflowStep::Choice { cases } => Box::new(cases.shrink().map(|cases| Self::Choice { cases })),
            }
        }
    }

    impl Arbitrary for Workflow<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                name: Arbitrary::arbitrary(g),
                args: Arbitrary::arbitrary(g),
                steps: Arbitrary::arbitrary(g),
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let args = shrink_struct!(self; name, steps; args);
            let steps = shrink_struct!(self; name, args; steps);
            Box::new(args.chain(steps))
        }
    }

    impl Arbitrary for Query<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            fn word(g: &mut Gen) -> String {
                static CHOICES: OnceCell<Vec<char>> = OnceCell::new();
                let choices = CHOICES.get_or_init(|| ('a'..='z').chain('A'..='Z').chain('0'..='9').collect());
                let len = Vec::<()>::arbitrary(g).len().max(1);
                (0..len).map(|_| g.choose(choices).unwrap()).collect()
            }
            let prev = CTX.with(|c| c.replace(Context::Simple { now: Timestamp::now() }));
            let source = Source::arbitrary(g);
            let depth = DEPTH.with(|d| *d.borrow());
            let events = if depth == 0 {
                Vec::<Ident>::arbitrary(g)
                    .into_iter()
                    .map(|label| {
                        let tags = Vec::<SingleTag>::arbitrary(g);
                        let t = Type::Atom(TypeAtom::Record(
                            NonEmptyVec::<Label>::arbitrary(g).map(|l| (l.clone(), Type::arbitrary(g))),
                        ));
                        (label, (t, tags))
                    })
                    .collect()
            } else {
                BTreeMap::new()
            };
            let workflows = if depth == 0 {
                Vec::<Workflow>::arbitrary(g)
                    .into_iter()
                    .map(|w| (w.name.clone(), w))
                    .collect()
            } else {
                BTreeMap::new()
            };
            let ret = Self {
                pragmas: Vec::new(),
                features: Vec::<bool>::arbitrary(g).into_iter().map(|_| word(g)).collect(),
                source,
                ops: Arbitrary::arbitrary(g),
                events: Arc::new(events),
                workflows: Arc::new(workflows),
            };
            CTX.with(|c| c.replace(prev));
            ret
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let features = shrink_struct!(self; pragmas, source, ops, events, workflows; features);
            let source = shrink_struct!(self; pragmas, features, ops, events, workflows; source);
            let ops = shrink_struct!(self; pragmas, features, source, events, workflows; ops);
            let events = shrink_struct!(self; pragmas, features, source, ops, workflows; events);
            let workflows = shrink_struct!(self; pragmas, features, source, ops, events; workflows);
            Box::new(features.chain(source).chain(ops).chain(events).chain(workflows))
        }
    }

    impl Arbitrary for SortKey {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                lamport: Arbitrary::arbitrary(g),
                stream: Arbitrary::arbitrary(g),
            }
        }
    }

    #[test]
    fn qc_roundtrip() {
        fn roundtrip_aql(q: Query<'static>) -> TestResult {
            // What this test currently ascertains is that our rendered string is isomorphic
            // to the internal representation of the parse tree, hence there are many “unnecessary”
            // parentheses in the output. If we want to remove those parentheses, we need to
            // formulate a proper canonicalisation strategy and apply it to the source query as well
            // as during parsing. Luckily, this test will then prove that our canonicalisation
            // actually works.
            let s = q.to_string();
            // println!(
            //     "q={:?}",
            //     &s[..s.char_indices().nth(50000).map(|x| x.0).unwrap_or(s.len())]
            // );
            // let mut buf = String::new();
            // stdin().read_line(&mut buf).unwrap();
            let p = match Query::parse(&s) {
                Ok(p) => p,
                Err(e) => return TestResult::error(format!("parse error: {e}\nq={s}")),
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
            .gen(Gen::new(5))
            .quickcheck(roundtrip_aql as fn(Query<'static>) -> TestResult)
    }
}
