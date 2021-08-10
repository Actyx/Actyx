#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]

use std::{convert::TryInto, str::FromStr, sync::Arc};

use super::{
    non_empty::NonEmptyVec, AggrOp, Arr, Ind, Index, Num, Obj, Operation, Query, SimpleExpr, TagAtom, TagExpr,
};
use crate::{language::SortKey, tags::Tag, Timestamp};
use anyhow::{bail, ensure, Result};
use chrono::{TimeZone, Utc};
use once_cell::sync::Lazy;
use pest::{prec_climber::PrecClimber, Parser};
use utils::*;

#[derive(Debug, Clone)]
pub struct NoVal(&'static str);
impl std::fmt::Display for NoVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no value was present for {}", self.0)
    }
}
impl std::error::Error for NoVal {}

#[derive(pest_derive::Parser)]
#[grammar = "language/aql.pest"]
struct Aql;

mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Simple,
    Aggregate,
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum ContextError {
    #[display(fmt = "aggregators are only valid in AGGREGATE clauses")]
    AggregatorOutsideAggregate,
    #[display(fmt = "current value _ not available in AGGREGATE clauses")]
    CurrentValueInAggregate,
}

fn r_tag(p: P) -> Result<Tag> {
    let quoted = p.single()?.single()?;
    let s = quoted.as_str();
    let s = &s[1..s.len() - 1];
    match quoted.as_rule() {
        Rule::single_quoted => Ok(Tag::from_str(s.replace("''", "'").as_ref())?),
        Rule::double_quoted => Ok(Tag::from_str(s.replace("\"\"", "\"").as_ref())?),
        x => bail!("unexpected token: {:?}", x),
    }
}

enum FromTo {
    From,
    To,
}
fn r_tag_from_to(p: P, f: FromTo) -> Result<TagExpr> {
    use TagAtom::*;
    use TagExpr::Atom;
    let mut p = p.inner()?;
    let mut first = p.next().ok_or(NoVal("r_tag_from_to first"))?;
    Ok(match first.as_rule() {
        Rule::natural => {
            let lamport = first.natural()?.into();
            // if no streamId was given, use the first one (just like assuming 00:00:00 for a date)
            let stream = p.parse_or_default()?;
            match f {
                FromTo::From => Atom(FromLamport(SortKey { lamport, stream })),
                FromTo::To => Atom(ToLamport(SortKey { lamport, stream })),
            }
        }
        Rule::isodate => match f {
            FromTo::From => Atom(FromTime(r_timestamp(first)?)),
            FromTo::To => Atom(ToTime(r_timestamp(first)?)),
        },
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_tag_expr(p: P) -> Result<TagExpr> {
    use TagAtom::*;
    use TagExpr::Atom;

    static CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
        use pest::prec_climber::{Assoc::*, Operator};

        PrecClimber::new(vec![Operator::new(Rule::or, Left), Operator::new(Rule::and, Left)])
    });

    CLIMBER.climb(
        p.inner()?,
        |p| {
            Ok(match p.as_rule() {
                Rule::tag => Atom(Tag(r_tag(p)?)),
                Rule::tag_expr => r_tag_expr(p)?,
                Rule::all_events => Atom(AllEvents),
                Rule::is_local => Atom(IsLocal),
                Rule::tag_from => r_tag_from_to(p, FromTo::From)?,
                Rule::tag_to => r_tag_from_to(p, FromTo::To)?,
                Rule::tag_app => Atom(AppId(p.single()?.as_str().parse()?)),
                x => bail!("unexpected token: {:?}", x),
            })
        },
        |lhs, op, rhs| {
            Ok(match op.as_rule() {
                Rule::and => lhs?.and(rhs?),
                Rule::or => lhs?.or(rhs?),
                x => bail!("unexpected token: {:?}", x),
            })
        },
    )
}

fn r_string(p: P) -> Result<String> {
    let p = p.single()?;
    Ok(match p.as_rule() {
        Rule::nonempty_string => {
            let p = p.single()?;
            let s = p.as_str();
            let s = &s[1..s.len() - 1];
            match p.as_rule() {
                Rule::single_quoted => s.replace("''", "'"),
                Rule::double_quoted => s.replace("\"\"", "\""),
                x => bail!("unexpected token: {:?}", x),
            }
        }
        Rule::empty_string => String::new(),
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_var(p: P, ctx: Context) -> Result<SimpleExpr> {
    let mut p = p.inner()?;
    let s = p.string()?;
    if s == "_" {
        ensure!(ctx != Context::Aggregate, ContextError::CurrentValueInAggregate);
    }
    let head = SimpleExpr::Variable(s.try_into()?);
    let mut tail = vec![];
    for mut i in p {
        match i.as_rule() {
            Rule::ident => tail.push(Index::String(i.string()?)),
            Rule::natural => tail.push(Index::Number(i.natural()?)),
            Rule::string => tail.push(Index::String(r_string(i)?)),
            Rule::simple_expr => tail.push(Index::Expr(r_simple_expr(i, ctx)?)),
            x => bail!("unexpected token: {:?}", x),
        }
    }
    Ok(if tail.is_empty() {
        head
    } else {
        SimpleExpr::Indexing(Ind {
            head: Arc::new(head),
            tail: tail.try_into()?,
        })
    })
}

fn r_expr_index(p: P, ctx: Context) -> Result<SimpleExpr> {
    let mut p = p.inner()?;
    let head = r_simple_expr(p.next().ok_or(NoVal("r_expr_index head"))?, ctx)?;
    let mut tail = vec![];
    for mut i in p {
        match i.as_rule() {
            Rule::ident => tail.push(Index::String(i.string()?)),
            Rule::natural => tail.push(Index::Number(i.natural()?)),
            Rule::string => tail.push(Index::String(r_string(i)?)),
            Rule::simple_expr => tail.push(Index::Expr(r_simple_expr(i, ctx)?)),
            x => bail!("unexpected token: {:?}", x),
        }
    }
    Ok(if tail.is_empty() {
        head
    } else {
        SimpleExpr::Indexing(Ind {
            head: Arc::new(head),
            tail: tail.try_into()?,
        })
    })
}

fn r_number(mut p: P) -> Result<Num> {
    p.natural().map(Num::Natural).or_else(|_| p.decimal().map(Num::Decimal))
}

fn r_timestamp(p: P) -> Result<Timestamp> {
    let mut p = p.inner()?;
    let year: i32 = p.string()?.parse()?;
    let month: u32 = p.string()?.parse()?;
    let day: u32 = p.string()?.parse()?;
    let hour: u32 = p.parse_or_default()?;
    let min: u32 = p.parse_or_default()?;
    let sec: u32 = p.parse_or_default()?;
    let nano: u32 = if let Some(p) = p.next() {
        match p.as_rule() {
            Rule::millisecond => p.as_str().parse::<u32>()? * 1_000_000,
            Rule::microsecond => p.as_str().parse::<u32>()? * 1_000,
            Rule::nanosecond => p.as_str().parse::<u32>()?,
            x => bail!("unexpected token: {:?}", x),
        }
    } else {
        0
    };
    Ok(Utc.ymd(year, month, day).and_hms_nano(hour, min, sec, nano).into())
}

fn r_object(p: P, ctx: Context) -> Result<Obj> {
    let mut props = vec![];
    let mut p = p.inner()?;
    while p.peek().is_some() {
        let key = {
            let mut i = p.next().ok_or(NoVal("key"))?;
            match i.as_rule() {
                Rule::ident => Index::String(i.string()?),
                Rule::natural => Index::Number(i.natural()?),
                Rule::string => Index::String(r_string(i)?),
                Rule::simple_expr => Index::Expr(r_simple_expr(i, ctx)?),
                x => bail!("unexpected token: {:?}", x),
            }
        };
        let value = r_simple_expr(p.next().ok_or(NoVal("value"))?, ctx)?;
        props.push((key, value));
    }
    Ok(Obj { props: props.into() })
}

fn r_array(p: P, ctx: Context) -> Result<Arr> {
    Ok(Arr {
        items: p.inner()?.map(|p| r_simple_expr(p, ctx)).collect::<Result<_>>()?,
    })
}

fn r_bool(p: P) -> bool {
    p.as_str() == "TRUE"
}

fn r_cases(p: P, ctx: Context) -> Result<NonEmptyVec<(SimpleExpr, SimpleExpr)>> {
    let mut p = p.inner()?;
    let mut ret = Vec::new();
    while let Some(pred) = p.next() {
        let pred = r_simple_expr(pred, ctx)?;
        let expr = r_simple_expr(p.next().ok_or(NoVal("case expression"))?, ctx)?;
        ret.push((pred, expr));
    }
    Ok(ret.try_into()?)
}

fn r_not(p: P) -> Result<P> {
    p.single()
}

fn r_aggr(p: P, ctx: Context) -> Result<SimpleExpr> {
    ensure!(ctx == Context::Aggregate, ContextError::AggregatorOutsideAggregate);
    let p = p.single()?;
    Ok(match p.as_rule() {
        Rule::aggr_sum => SimpleExpr::AggrOp(Arc::new((AggrOp::Sum, r_simple_expr(p.single()?, Context::Simple)?))),
        Rule::aggr_prod => SimpleExpr::AggrOp(Arc::new((AggrOp::Prod, r_simple_expr(p.single()?, Context::Simple)?))),
        Rule::aggr_min => SimpleExpr::AggrOp(Arc::new((AggrOp::Min, r_simple_expr(p.single()?, Context::Simple)?))),
        Rule::aggr_max => SimpleExpr::AggrOp(Arc::new((AggrOp::Max, r_simple_expr(p.single()?, Context::Simple)?))),
        Rule::aggr_first => SimpleExpr::AggrOp(Arc::new((AggrOp::First, r_simple_expr(p.single()?, Context::Simple)?))),
        Rule::aggr_last => SimpleExpr::AggrOp(Arc::new((AggrOp::Last, r_simple_expr(p.single()?, Context::Simple)?))),
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_simple_expr(p: P, ctx: Context) -> Result<SimpleExpr> {
    static CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
        use pest::prec_climber::{Assoc::*, Operator};
        let op = Operator::new;

        PrecClimber::new(vec![
            op(Rule::alternative, Left),
            op(Rule::or, Left),
            op(Rule::xor, Left),
            op(Rule::and, Left),
            op(Rule::eq, Left) | op(Rule::ne, Left),
            op(Rule::lt, Left) | op(Rule::le, Left) | op(Rule::gt, Left) | op(Rule::ge, Left),
            op(Rule::add, Left) | op(Rule::sub, Left),
            op(Rule::mul, Left) | op(Rule::div, Left) | op(Rule::modulo, Left),
            op(Rule::pow, Left),
        ])
    });

    fn primary(p: P, ctx: Context) -> Result<SimpleExpr> {
        Ok(match p.as_rule() {
            Rule::decimal => SimpleExpr::Number(r_number(p)?),
            Rule::var_index => r_var(p, ctx)?,
            Rule::expr_index => r_expr_index(p, ctx)?,
            Rule::simple_expr => r_simple_expr(p, ctx)?,
            Rule::simple_not => SimpleExpr::Not(primary(r_not(p)?, ctx)?.into()),
            Rule::string => SimpleExpr::String(r_string(p)?),
            Rule::object => SimpleExpr::Object(r_object(p, ctx)?),
            Rule::array => SimpleExpr::Array(r_array(p, ctx)?),
            Rule::null => SimpleExpr::Null,
            Rule::bool => SimpleExpr::Bool(r_bool(p)),
            Rule::simple_cases => SimpleExpr::Cases(r_cases(p, ctx)?),
            Rule::aggr_op => r_aggr(p, ctx)?,
            x => bail!("unexpected token: {:?}", x),
        })
    }

    CLIMBER.climb(
        p.inner()?,
        |p| primary(p, ctx),
        |lhs, op, rhs| {
            Ok(match op.as_rule() {
                Rule::add => lhs?.add(rhs?),
                Rule::sub => lhs?.sub(rhs?),
                Rule::mul => lhs?.mul(rhs?),
                Rule::div => lhs?.div(rhs?),
                Rule::modulo => lhs?.modulo(rhs?),
                Rule::pow => lhs?.pow(rhs?),
                Rule::and => lhs?.and(rhs?),
                Rule::or => lhs?.or(rhs?),
                Rule::xor => lhs?.xor(rhs?),
                Rule::lt => lhs?.lt(rhs?),
                Rule::le => lhs?.le(rhs?),
                Rule::gt => lhs?.gt(rhs?),
                Rule::ge => lhs?.ge(rhs?),
                Rule::eq => lhs?.eq(rhs?),
                Rule::ne => lhs?.ne(rhs?),
                Rule::alternative => lhs?.alt(rhs?),
                x => bail!("unexpected token: {:?}", x),
            })
        },
    )
}

fn r_query(features: Vec<String>, p: P) -> Result<Query> {
    let mut p = p.inner()?;
    let mut q = Query {
        features,
        from: r_tag_expr(p.next().ok_or(NoVal("tag expression"))?)?,
        ops: vec![],
    };
    for o in p {
        match o.as_rule() {
            Rule::filter => q
                .ops
                .push(Operation::Filter(r_simple_expr(o.single()?, Context::Simple)?)),
            Rule::select => {
                let v = o
                    .inner()?
                    .map(|p| r_simple_expr(p, Context::Simple))
                    .collect::<Result<Vec<_>>>()?;
                q.ops.push(Operation::Select(v.try_into()?))
            }
            Rule::aggregate => q
                .ops
                .push(Operation::Aggregate(r_simple_expr(o.single()?, Context::Aggregate)?)),
            x => bail!("unexpected token: {:?}", x),
        }
    }
    Ok(q)
}

impl FromStr for Query {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut p = Aql::parse(Rule::main_query, s)?.single()?.inner()?;
        let mut f = p.next().ok_or(NoVal("main query"))?;
        let features = if f.as_rule() == Rule::features {
            let features = f.inner()?.map(|mut ff| ff.string()).collect::<Result<_>>()?;
            f = p.next().ok_or(NoVal("FROM"))?;
            features
        } else {
            vec![]
        };
        r_query(features, f)
    }
}

impl FromStr for TagExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_tag_expr, s)?.single()?.single()?;
        r_tag_expr(p)
    }
}

impl FromStr for SimpleExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_simple_expr, s)?.single()?.single()?;
        r_simple_expr(p, Context::Simple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{language::var::Var, tag, NodeId, StreamId};
    use pest::{fails_with, Parser};
    use std::convert::TryFrom;

    #[test]
    fn tag() -> Result<()> {
        let p = Aql::parse(Rule::tag, "'hello''s revenge'")?;
        assert_eq!(r_tag(p.single()?)?, tag!("hello's revenge"));
        let p = Aql::parse(Rule::tag, "\"hello\"\"s revenge\"")?;
        assert_eq!(r_tag(p.single()?)?, tag!("hello\"s revenge"));
        Ok(())
    }

    #[test]
    fn tag_expr() -> Result<()> {
        use TagAtom::Tag;
        use TagExpr::*;
        assert_eq!(
            "'x' |\t'y'\n&'z'".parse::<TagExpr>()?,
            Or((
                Atom(Tag(tag!("x"))),
                And((Atom(Tag(tag!("y"))), Atom(Tag(tag!("z")))).into())
            )
                .into())
        );
        Ok(())
    }

    #[test]
    fn simple_expr() -> Result<()> {
        use super::{Num::*, SimpleExpr::*};
        assert_eq!(
            "(x - 5.2 * 1234)^2 / 7 % 5".parse::<SimpleExpr>()?,
            Variable("x".try_into()?)
                .sub(Number(Decimal(5.2)).mul(Number(Natural(1234))))
                .pow(Number(Natural(2)))
                .div(Number(Natural(7)))
                .modulo(Number(Natural(5)))
        );

        fails_with! {
            parser: Aql,
            input: "5+3!",
            rule: Rule::main_simple_expr,
            positives: vec![Rule::EOI, Rule::add, Rule::sub, Rule::mul, Rule::div, Rule::modulo, Rule::pow, Rule::and,
                Rule::or, Rule::xor, Rule::lt, Rule::le, Rule::gt, Rule::ge, Rule::eq, Rule::ne, Rule::alternative],
            negatives: vec![],
            pos: 3
        };

        Ok(())
    }

    #[test]
    fn query() -> Result<()> {
        use super::Num::*;
        use super::{Arr, Ind, Obj};
        use crate::app_id;
        use SimpleExpr::*;
        use TagAtom::*;
        use TagExpr::Atom;

        assert_eq!(
            "FROM 'machine' | 'user' END".parse::<Query>()?,
            Query::new(Tag(tag!("machine")).or(Tag(tag!("user"))))
        );
        assert_eq!(
            "FROM 'machine' |
                -- or the other
                  'user' & isLocal & from(2012-12-31Z) & to(12345678901234567) & \
                  from(10/1234567890123456789012345678901234567890122-4312) & appId(hello-5.-x-) & allEvents
                  FILTER _.x[42] > 5 SELECT { x: ! 'hello' y: 42 z: [1.3,_.x] } END --"
                .parse::<Query>()?,
            Query::new(
                Atom(Tag(tag!("machine"))).or(Tag(tag!("user"))
                    .and(IsLocal)
                    .and(Atom(FromTime(1356912000000000.into())))
                    .and(Atom(ToLamport(SortKey {
                        lamport: 12345678901234567.into(),
                        stream: StreamId::min(),
                    })))
                    .and(Atom(FromLamport(SortKey {
                        lamport: 10.into(),
                        stream: NodeId([
                            12, 65, 70, 28, 130, 74, 44, 32, 196, 20, 97, 200, 36, 162, 194, 12, 65, 70, 28, 130, 74,
                            44, 32, 196, 20, 97, 200, 36, 162, 194, 12, 65
                        ])
                        .stream(4312.into())
                    })))
                    .and(Atom(AppId(app_id!("hello-5.-x-"))))
                    .and(Atom(AllEvents)))
            )
            .with_op(Operation::Filter(Ind::with("_", &[&"x", &42]).gt(Number(Natural(5)))))
            .with_op(Operation::Select(
                vec![Obj::with(&[
                    ("x", Not(String("hello".to_owned()).into())),
                    ("y", Number(Natural(42))),
                    ("z", Arr::with(&[Number(Decimal(1.3)), Ind::with("_", &[&"x"])]))
                ])]
                .try_into()?
            ))
        );
        Ok(())
    }

    #[test]
    fn positive() {
        let p = |str: &'static str| str.parse::<Query>().unwrap();
        p("FROM 'machine' | 'user' & isLocal & from(2012-12-31Z) & to(12345678901234567) & appId(hello-5.-x-) & allEvents FILTER _.x[42] > 5 SELECT { x: !'hello', y: 42, z: [1.3, _.x] } END");
        p("FROM from(2012-12-31T09:30:32.007Z) END");
        p("FROM from(2012-12-31T09:30:32Z) END");
        p("FROM from(2012-12-31T09:30:32.007008Z) END");
        p("FROM 'hello''s revenge' END");
        p("FROM 'hell''o' FILTER _.x = 'worl''d' END");
        p("FROM 'a' & 'b' | 'c' END");
        p("FROM 'a' | 'b' & 'c' END");
        p("FROM 'a' & ('b' | 'c') END");
        p("FROM 'a' & 'b' | 'c' & 'd' END");
        p("FROM ('a' | 'b') & ('c' | 'd') END");
    }

    #[test]
    fn negative() {
        fails_with! {
            parser: Aql,
            input: "FROM x",
            rule: Rule::main_query,
            positives: vec![Rule::tag_expr],
            negatives: vec![],
            pos: 5
        };
        fails_with! {
            parser: Aql,
            input: "FROM 'x' ELECT 'x'",
            rule: Rule::main_query,
            positives: vec![Rule::EOI, Rule::filter, Rule::select, Rule::aggregate, Rule::and, Rule::or],
            negatives: vec![],
            pos: 9
        };
        fails_with! {
            parser: Aql,
            input: "FROM 'x' FITTER 'x'",
            rule: Rule::main_query,
            positives: vec![Rule::EOI, Rule::filter, Rule::select, Rule::aggregate, Rule::and, Rule::or],
            negatives: vec![],
            pos: 9
        };
    }

    #[test]
    fn expr() {
        use super::Num::*;
        use SimpleExpr::*;
        let p = |s: &'static str| s.parse::<SimpleExpr>().unwrap();
        assert_eq!(p("NULL"), Null);
        assert_eq!(p("FALSE"), Bool(false));
        assert_eq!(p("1"), Number(Natural(1)));
        assert_eq!(p("1.0"), Number(Natural(1)));
        assert_eq!(p("'s'"), String("s".into()));
        assert_eq!(
            p("[1,TRUE]"),
            Array(Arr {
                items: vec![Number(Natural(1)), Bool(true)].into()
            })
        );
        assert_eq!(
            p("{one:1 ['two']:2 [('three')]:3 [4]:4}"),
            Object(Obj {
                props: vec![
                    (Index::String("one".into()), Number(Natural(1))),
                    (Index::String("two".into()), Number(Natural(2))),
                    (Index::Expr(String("three".into())), Number(Natural(3))),
                    (Index::Number(4), Number(Natural(4))),
                ]
                .into()
            })
        );
    }

    #[test]
    fn ident() {
        let p = |s: &str, e: Option<&str>| {
            let q = s.parse::<Query>();
            if let Some(err) = e {
                let e = q.unwrap_err().to_string();
                assert!(e.contains(err), "received: {}", e);
                None
            } else {
                match q.unwrap().ops[0].clone() {
                    Operation::Select(v) => Some(v.to_vec()),
                    _ => None,
                }
            }
        };
        let ind = |s: &str| {
            SimpleExpr::Indexing(Ind {
                head: Arc::new(SimpleExpr::Variable(Var::try_from("_").unwrap())),
                tail: NonEmptyVec::try_from(vec![Index::String(s.to_owned())]).unwrap(),
            })
        };
        let s = |s: &str| Index::String(s.to_owned());
        let n = |n: u64| SimpleExpr::Number(Num::Natural(n));
        let v = |s: &str| SimpleExpr::Variable(Var::try_from(s).unwrap());

        p("FROM 'x' SELECT _.H", Some("expected ident"));
        p("FROM 'x' SELECT _.HE", Some("expected ident"));
        assert_eq!(
            p("FROM 'x' SELECT i, iIö, PσΔ", None),
            Some(vec![v("i"), v("iIö"), v("PσΔ")])
        );
        assert_eq!(
            p("FROM 'x' SELECT _.i, _.iIö, _.PσΔ", None),
            Some(vec![ind("i"), ind("iIö"), ind("PσΔ")])
        );
        assert_eq!(
            p("FROM 'x' SELECT { i: 1 iIö: 2 PσΔ: 3 }", None),
            Some(vec![SimpleExpr::Object(Obj {
                props: Arc::from(vec![(s("i"), n(1)), (s("iIö"), n(2)), (s("PσΔ"), n(3))].as_slice())
            })])
        )
    }

    #[test]
    fn aggregate() {
        let p = |s: &str, e: Option<&str>| {
            let q = s.parse::<Query>();
            if let Some(err) = e {
                let e = q.unwrap_err().to_string();
                assert!(e.contains(err), "received: {}", e);
            } else {
                q.unwrap();
            }
        };

        p(
            "FROM 'x' FILTER SUM(1)",
            Some("aggregators are only valid in AGGREGATE clauses"),
        );
        p(
            "FROM 'x' SELECT 1 + SUM(1)",
            Some("aggregators are only valid in AGGREGATE clauses"),
        );
        p("FROM 'x' FILTER _", None);
        p(
            "FROM 'x' AGGREGATE 1 + _",
            Some("current value _ not available in AGGREGATE clauses"),
        );
        p("FROM 'x' AGGREGATE 1 + 2", None);
        p(
            "FROM 'x' AGGREGATE { a: LAST(_ + 1) b: FIRST(_.a.b) c: MIN(1 / _) d: MAX([_]) e: SUM(1.3 * _) }",
            None,
        );
    }
}
