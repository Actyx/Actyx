//! syntax trees need rewrites
//!
//! There are basically two ways of doing this:
//! - tree is fully owned and gets fully copied
//! - tree is dynamically shared and gets partially copied
//!
//! We choose the second case, which implies that a tree must be immutable.
//! The benefit is structural sharing, i.e. minimal copying, so we need to be
//! careful to design the API such that this goal is achieved, lest the effort
//! be for naught.
use super::*;

pub enum Tactic<T, D: ?Sized> {
    KeepAsIs,
    Scrutinise,
    Replace(T),
    ReplaceLater(fn(&mut D, T) -> (T, bool)),
}

pub trait Galactus {
    fn visit_tag_atom(&mut self, tag: &TagAtom) -> Tactic<TagAtom, Self>;
    fn visit_expr(&mut self, expr: &SimpleExpr) -> Tactic<SimpleExpr, Self>;
}

impl<'a> Query<'a> {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        let (source, mut changed) = self.source.rewrite(surfer);
        let ops = self
            .ops
            .iter()
            .map(|op| shed(op.rewrite(surfer), &mut changed))
            .collect();
        emit(
            || Self {
                pragmas: self.pragmas.clone(),
                features: self.features.clone(),
                source,
                ops,
            },
            changed,
            self,
        )
    }
}

impl Source {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match self {
            Source::Events { from, order } => {
                let (from, changed) = from.rewrite(surfer);
                (Source::Events { from, order: *order }, changed)
            }
            Source::Array(arr) => {
                let mut items = Vec::with_capacity(arr.items.len());
                let mut changed = false;
                for item in arr.items.iter() {
                    items.push(shed(item.rewrite_spread(surfer), &mut changed));
                }
                emit(|| Source::Array(Arr { items: items.into() }), changed, self)
            }
        }
    }
}

impl Operation {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match self {
            Operation::Filter(x) => map(x.rewrite(surfer), Operation::Filter),
            Operation::Select(x) => {
                let mut changed = false;
                let exprs = x.map(|s| shed(s.rewrite_spread(surfer), &mut changed));
                emit(|| Operation::Select(exprs), changed, self)
            }
            Operation::Aggregate(x) => map(x.rewrite(surfer), Operation::Aggregate),
            Operation::Limit(x) => (Operation::Limit(*x), false),
            Operation::Binding(x, y) => map(y.rewrite(surfer), |y| Operation::Binding(x.clone(), y)),
        }
    }
}

impl TagExpr {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match self {
            TagExpr::Or(x) => {
                let mut changed = false;
                let l = shed(x.0.rewrite(surfer), &mut changed);
                let r = shed(x.1.rewrite(surfer), &mut changed);
                emit(|| TagExpr::Or(Arc::new((l, r))), changed, self)
            }
            TagExpr::And(x) => {
                let mut changed = false;
                let l = shed(x.0.rewrite(surfer), &mut changed);
                let r = shed(x.1.rewrite(surfer), &mut changed);
                emit(|| TagExpr::And(Arc::new((l, r))), changed, self)
            }
            TagExpr::Atom(x) => match surfer.visit_tag_atom(x) {
                Tactic::KeepAsIs => (self.clone(), false),
                Tactic::Scrutinise => map(x.rewrite(surfer), TagExpr::Atom),
                Tactic::Replace(atom) => (TagExpr::Atom(atom), true),
                Tactic::ReplaceLater(f) => {
                    let (atom, mut changed) = x.rewrite(surfer);
                    let atom = shed((f)(surfer, atom), &mut changed);
                    emit(|| TagExpr::Atom(atom), changed, self)
                }
            },
        }
    }
}

impl TagAtom {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match self {
            TagAtom::Interpolation(x) => {
                let mut changed = false;
                let items = x
                    .items
                    .iter()
                    .map(|expr| shed(expr.rewrite(surfer), &mut changed))
                    .collect();
                emit(|| TagAtom::Interpolation(Arr { items }), changed, self)
            }
            TagAtom::Tag(_)
            | TagAtom::AllEvents
            | TagAtom::IsLocal
            | TagAtom::FromTime(_, _)
            | TagAtom::ToTime(_, _)
            | TagAtom::FromLamport(_, _)
            | TagAtom::ToLamport(_, _)
            | TagAtom::AppId(_) => (self.clone(), false),
        }
    }
}

impl SpreadExpr {
    pub fn rewrite_spread(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        map(self.expr.rewrite(surfer), |expr| expr.with_spread(self.spread))
    }
}

impl Index {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match self {
            Index::Expr(e) => map(e.rewrite(surfer), Index::Expr),
            Index::String(_) | Index::Number(_) => (self.clone(), false),
        }
    }
}

impl SimpleExpr {
    pub fn rewrite(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        match surfer.visit_expr(self) {
            Tactic::KeepAsIs => (self.clone(), false),
            Tactic::Scrutinise => self.rewrite0(surfer),
            Tactic::Replace(expr) => (expr, true),
            Tactic::ReplaceLater(f) => {
                let (expr, mut changed) = self.rewrite0(surfer);
                let expr = shed((f)(surfer, expr), &mut changed);
                emit(|| expr, changed, self)
            }
        }
    }

    fn rewrite0(&self, surfer: &mut impl Galactus) -> (Self, bool) {
        let mut changed = false;
        match self {
            SimpleExpr::Indexing(Ind { head, tail }) => {
                let head = shed(head.rewrite(surfer), &mut changed);
                let tail = tail.map(|i| shed(i.rewrite(surfer), &mut changed));
                emit(
                    || {
                        let head = Arc::new(head);
                        SimpleExpr::Indexing(Ind { head, tail })
                    },
                    changed,
                    self,
                )
            }
            SimpleExpr::Interpolation(x) => {
                let v = x.iter().map(|expr| shed(expr.rewrite(surfer), &mut changed)).collect();
                (SimpleExpr::Interpolation(v), changed)
            }
            SimpleExpr::Object(Obj { props }) => {
                let props = props
                    .iter()
                    .map(|(i, e)| {
                        let i = shed(i.rewrite(surfer), &mut changed);
                        let e = shed(e.rewrite(surfer), &mut changed);
                        (i, e)
                    })
                    .collect();
                emit(|| SimpleExpr::Object(Obj { props }), changed, self)
            }
            SimpleExpr::Array(Arr { items }) => {
                let items = items
                    .iter()
                    .map(|e| shed(e.rewrite_spread(surfer), &mut changed))
                    .collect();
                emit(|| SimpleExpr::Array(Arr { items }), changed, self)
            }
            SimpleExpr::Cases(c) => {
                let c = c.map(|(cond, expr)| {
                    (
                        shed(cond.rewrite(surfer), &mut changed),
                        shed(expr.rewrite(surfer), &mut changed),
                    )
                });
                emit(|| SimpleExpr::Cases(c), changed, self)
            }
            SimpleExpr::BinOp(o) => {
                let l = shed(o.1.rewrite(surfer), &mut changed);
                let r = shed(o.2.rewrite(surfer), &mut changed);
                emit(|| SimpleExpr::BinOp(Arc::new((o.0, l, r))), changed, self)
            }
            SimpleExpr::Not(e) => {
                let e = shed(e.rewrite(surfer), &mut changed);
                emit(|| SimpleExpr::Not(Arc::new(e)), changed, self)
            }
            SimpleExpr::AggrOp(a) => {
                let expr = shed(a.1.rewrite(surfer), &mut changed);
                emit(|| SimpleExpr::AggrOp(Arc::new((a.0, expr))), changed, self)
            }
            SimpleExpr::FuncCall(FuncCall { name, args }) => {
                let args = args.iter().map(|e| shed(e.rewrite(surfer), &mut changed)).collect();
                emit(
                    || {
                        let name = name.clone();
                        SimpleExpr::FuncCall(FuncCall { name, args })
                    },
                    changed,
                    self,
                )
            }
            SimpleExpr::SubQuery(q) => map(q.rewrite(surfer), SimpleExpr::SubQuery),
            SimpleExpr::KeyVar(_)
            | SimpleExpr::KeyLiteral(_)
            | SimpleExpr::TimeVar(_)
            | SimpleExpr::TimeLiteral(_)
            | SimpleExpr::Tags(_)
            | SimpleExpr::App(_)
            | SimpleExpr::Variable(_)
            | SimpleExpr::Number(_)
            | SimpleExpr::String(_)
            | SimpleExpr::Null
            | SimpleExpr::Bool(_) => (self.clone(), false),
        }
    }
}

fn emit<T: Clone>(computed: impl FnOnce() -> T, changed: bool, original: &T) -> (T, bool) {
    if changed {
        ((computed)(), true)
    } else {
        (original.clone(), false)
    }
}

fn map<T, U>(x: (T, bool), f: impl FnOnce(T) -> U) -> (U, bool) {
    (f(x.0), x.1)
}

fn shed<T>(x: (T, bool), b: &mut bool) -> T {
    *b |= x.1;
    x.0
}
