#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]

use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
    sync::Arc,
};

use super::{
    non_empty::NonEmptyVec, AggrOp, Arr, FuncCall, Ind, Index, Num, Obj, Operation, Query, SimpleExpr, Source,
    SpreadExpr, TagAtom, TagExpr,
};
use crate::SortKey;
use anyhow::{bail, ensure, Result};
use ax_types::{service::Order, Tag, Timestamp};
use chrono::{FixedOffset, TimeZone, Timelike, Utc};
use once_cell::sync::Lazy;
use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
    Parser,
};
use unicode_normalization::UnicodeNormalization;
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
pub(crate) enum Context {
    Simple { now: Timestamp },
    Aggregate { now: Timestamp },
}

impl Context {
    pub(crate) fn now(&self) -> Timestamp {
        match self {
            Context::Simple { now } => *now,
            Context::Aggregate { now } => *now,
        }
    }

    pub(crate) fn is_aggregate(&self) -> bool {
        matches!(self, Self::Aggregate { .. })
    }

    pub(crate) fn aggregate(self) -> Self {
        match self {
            Context::Simple { now } => Context::Aggregate { now },
            Context::Aggregate { now } => Context::Aggregate { now },
        }
    }

    pub(crate) fn simple(self) -> Self {
        match self {
            Context::Simple { now } => Context::Simple { now },
            Context::Aggregate { now } => Context::Simple { now },
        }
    }
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum ContextError {
    #[display(fmt = "aggregators are only valid in AGGREGATE clauses")]
    AggregatorOutsideAggregate,
    #[display(fmt = "current value _ not available in AGGREGATE clauses")]
    CurrentValueInAggregate,
}

fn r_tag(p: P, ctx: Context) -> Result<TagExpr> {
    let tag = p.single()?;
    match tag.as_rule() {
        Rule::nonempty_string => {
            let quoted = tag.single()?;
            let s = quoted.as_str();
            let s = &s[1..s.len() - 1];
            match quoted.as_rule() {
                Rule::single_quoted => Ok(TagExpr::Atom(TagAtom::Tag(Tag::from_str(
                    s.replace("''", "'").as_ref(),
                )?))),
                Rule::double_quoted => Ok(TagExpr::Atom(TagAtom::Tag(Tag::from_str(
                    s.replace("\"\"", "\"").as_ref(),
                )?))),
                x => bail!("unexpected token: {:?}", x),
            }
        }
        Rule::interpolation => Ok(TagExpr::Atom(TagAtom::Interpolation(r_interpolation(tag, ctx)?))),
        x => bail!("unexpected token: {:?}", x),
    }
}

fn r_interpolation(p: P, ctx: Context) -> Result<Arr<SimpleExpr>> {
    let all = p.as_str();
    let mut e = p.inner()?;
    let mut pos = 1;
    let mut expr = Vec::new();
    let mut buffer = String::new();
    let end = all.len() - 1;
    while pos < end {
        if let Some(e) = e.next() {
            let brace = all[pos..].find('{').unwrap() + pos;
            buffer.push_str(&all[pos..brace]);
            let expr_len = e.as_str().len();
            match e.as_rule() {
                Rule::simple_expr => {
                    if !buffer.is_empty() {
                        expr.push(SimpleExpr::String(buffer));
                        buffer = String::new();
                    }
                    expr.push(r_simple_expr(e, ctx)?);
                }
                Rule::unicode => {
                    let c = char::from_u32(u32::from_str_radix(&e.as_str()[2..], 16)?)
                        .ok_or_else(|| anyhow::anyhow!("invalid unicode scalar value `{}`", e.as_str()))?;
                    buffer.push(c);
                }
                x => bail!("unexpected token: {:?}", x),
            }
            pos = brace + 1 + expr_len + 1;
        } else {
            buffer.push_str(&all[pos..end]);
            expr.push(SimpleExpr::String(buffer));
            buffer = String::new();
            break;
        }
    }
    if !buffer.is_empty() {
        expr.push(SimpleExpr::String(buffer));
    }
    let arr = Arr { items: expr.into() };
    Ok(arr)
}

enum FromTo {
    From(bool),
    To(bool),
}
fn r_tag_from_to(p: P, f: FromTo, ctx: Context) -> Result<TagExpr> {
    use TagAtom::*;
    use TagExpr::Atom;
    Ok(match p.as_rule() {
        Rule::event_key => {
            let mut p = p.inner()?;
            let lamport = p.natural()?.into();
            // if no streamId was given, use the first one (just like assuming 00:00:00 for a date)
            let stream = p.parse_or_default()?;
            match f {
                FromTo::From(incl) => Atom(FromLamport(SortKey { lamport, stream }, incl)),
                FromTo::To(incl) => Atom(ToLamport(SortKey { lamport, stream }, incl)),
            }
        }
        Rule::isodate => match f {
            FromTo::From(incl) => Atom(FromTime(r_timestamp(p)?, incl)),
            FromTo::To(incl) => Atom(ToTime(r_timestamp(p)?, incl)),
        },
        Rule::duration_ago => {
            let mut p = p.inner()?;
            let count = p.natural()?;
            let unit = match p.next().ok_or(NoVal("duration_unit"))?.as_str() {
                "s" => 1_000_000,
                "m" => 60_000_000,
                "h" => 3_600_000_000,
                "D" => 86_400_000_000,
                "W" => 604_800_000_000,    // seven days
                "M" => 2_551_442_876_908,  // Y2000 synodic month after Chapront(-Touzé)
                "Y" => 31_556_925_250_733, // J2000.0 mean tropical year
                x => bail!("unknown duration_unit {}", x),
            };
            let offset = count.saturating_mul(unit);
            let ts = ctx.now() - offset;
            match f {
                FromTo::From(incl) => Atom(FromTime(ts, incl)),
                FromTo::To(incl) => Atom(ToTime(ts, incl)),
            }
        }
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_tag_comp(p: P) -> Result<FromTo> {
    Ok(match p.as_rule() {
        Rule::lt => FromTo::To(false),
        Rule::le => FromTo::To(true),
        Rule::gt => FromTo::From(false),
        Rule::ge => FromTo::From(true),
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_tag_expr(p: P, ctx: Context) -> Result<TagExpr> {
    use TagAtom::*;
    use TagExpr::Atom;

    static PRATT: Lazy<PrattParser<Rule>> = Lazy::new(|| {
        PrattParser::new()
            .op(Op::infix(Rule::or, Assoc::Left))
            .op(Op::infix(Rule::and, Assoc::Left))
    });
    PRATT
        .map_primary(|p| {
            Ok(match p.as_rule() {
                Rule::tag => r_tag(p, ctx)?,
                Rule::tag_expr => r_tag_expr(p, ctx)?,
                Rule::all_events => Atom(AllEvents),
                Rule::is_local => Atom(IsLocal),
                Rule::tag_from => r_tag_from_to(p.single()?, FromTo::From(true), ctx)?,
                Rule::tag_to => r_tag_from_to(p.single()?, FromTo::To(false), ctx)?,
                Rule::tag_app => Atom(AppId(p.single()?.as_str().parse()?)),
                Rule::tag_key => {
                    let mut p = p.inner()?;
                    let from_to = r_tag_comp(p.next().ok_or(NoVal("tag_key first"))?)?;
                    r_tag_from_to(p.next().ok_or(NoVal("tag_key second"))?, from_to, ctx)?
                }
                Rule::tag_time => {
                    let mut p = p.inner()?;
                    let from_to = r_tag_comp(p.next().ok_or(NoVal("tag_time first"))?)?;
                    r_tag_from_to(p.next().ok_or(NoVal("tag_time second"))?, from_to, ctx)?
                }
                x => bail!("unexpected token: {:?}", x),
            })
        })
        .map_infix(|lhs, op, rhs| {
            Ok(match op.as_rule() {
                Rule::and => lhs?.and(rhs?),
                Rule::or => lhs?.or(rhs?),
                x => bail!("unexpected token: {:?}", x),
            })
        })
        .parse(p.inner()?)
}

fn r_order(p: P) -> Result<Order> {
    let p = p.single()?;
    match p.as_rule() {
        Rule::order => match p.as_str() {
            "ASC" => Ok(Order::Asc),
            "DESC" => Ok(Order::Desc),
            "STREAM" => Ok(Order::StreamAsc),
            x => bail!("unexpected order: {:?}", x),
        },
        x => bail!("unexpected token: {:?}", x),
    }
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

fn r_var(p: P, ctx: Context) -> Result<super::var::Var> {
    let s = p.as_str();
    if s == "_" {
        ensure!(!ctx.is_aggregate(), ContextError::CurrentValueInAggregate);
    }
    Ok(super::var::Var(s.nfc().collect()))
}

fn r_var_index(p: P, ctx: Context) -> Result<SimpleExpr> {
    let mut p = p.inner()?;
    let s = p.next().ok_or(NoVal("no var"))?.as_str();
    if s == "_" {
        ensure!(!ctx.is_aggregate(), ContextError::CurrentValueInAggregate);
    }
    let head = SimpleExpr::Variable(super::var::Var(s.nfc().collect()));
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
    let mut hour = 0u32;
    let mut min = 0u32;
    let mut sec = 0u32;
    let mut nano = 0u32;
    if p.peek().map(|r| r.as_rule()) == Some(Rule::hour) {
        hour = p.parse_or_default()?;
        min = p.parse_or_default()?;
        if p.peek().map(|p| p.as_rule()) == Some(Rule::second) {
            sec = p.parse_or_default()?;
        }
        nano = match p.peek().map(|p| p.as_rule()) {
            Some(Rule::millisecond) => p.parse_or_default::<u32>()? * 1_000_000,
            Some(Rule::microsecond) => p.parse_or_default::<u32>()? * 1_000,
            Some(Rule::nanosecond) => p.parse_or_default::<u32>()?,
            Some(Rule::sign) | None => 0,
            x => bail!("unexpected token: {:?}", x),
        }
    }
    if let Some(sign) = p.next() {
        let offset_hour: u32 = p.parse_or_default()?;
        let offset_min: u32 = p.parse_or_default()?;
        let mut seconds = offset_hour as i32 * 3600 + offset_min as i32 * 60;
        if sign.as_str() == "-" {
            seconds = -seconds;
        }
        Ok(FixedOffset::east_opt(seconds)
            .expect("valid by construction above")
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .single()
            .expect("ensured by the grammar")
            .with_nanosecond(nano)
            .expect("ensured by the grammar")
            .into())
    } else {
        Ok(Utc
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .single()
            .expect("ensured by the grammar")
            .with_nanosecond(nano)
            .expect("ensured by the grammar")
            .into())
    }
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

fn r_array(p: P, ctx: Context) -> Result<Arr<SpreadExpr>> {
    let mut p = p.inner()?;
    let mut items = Vec::new();
    while let Some(candidate) = p.next() {
        match candidate.as_rule() {
            Rule::spread => items.push(SpreadExpr {
                expr: r_simple_expr(p.next().unwrap(), ctx)?,
                spread: true,
            }),
            Rule::simple_expr => items.push(SpreadExpr {
                expr: r_simple_expr(candidate, ctx)?,
                spread: false,
            }),
            x => bail!("unexpected token: {:?}", x),
        }
    }
    Ok(Arr { items: items.into() })
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

fn r_aggr(p: P, ctx: Context) -> Result<SimpleExpr> {
    ensure!(ctx.is_aggregate(), ContextError::AggregatorOutsideAggregate);
    let p = p.single()?;
    Ok(match p.as_rule() {
        Rule::aggr_sum => SimpleExpr::AggrOp(Arc::new((AggrOp::Sum, r_simple_expr(p.single()?, ctx.simple())?))),
        Rule::aggr_prod => SimpleExpr::AggrOp(Arc::new((AggrOp::Prod, r_simple_expr(p.single()?, ctx.simple())?))),
        Rule::aggr_min => SimpleExpr::AggrOp(Arc::new((AggrOp::Min, r_simple_expr(p.single()?, ctx.simple())?))),
        Rule::aggr_max => SimpleExpr::AggrOp(Arc::new((AggrOp::Max, r_simple_expr(p.single()?, ctx.simple())?))),
        Rule::aggr_first => SimpleExpr::AggrOp(Arc::new((AggrOp::First, r_simple_expr(p.single()?, ctx.simple())?))),
        Rule::aggr_last => SimpleExpr::AggrOp(Arc::new((AggrOp::Last, r_simple_expr(p.single()?, ctx.simple())?))),
        x => bail!("unexpected token: {:?}", x),
    })
}

fn r_func_call(p: P, ctx: Context) -> Result<FuncCall> {
    let mut p = p.inner()?;
    let name = p.string()?;
    let mut args = vec![];
    for p in p {
        args.push(r_simple_expr(p, ctx)?);
    }
    Ok(FuncCall {
        name,
        args: args.into(),
    })
}

fn r_pragma(p: P) -> Result<(&str, &str)> {
    let mut p = p.inner()?;
    let name = p.next().unwrap().as_str();
    let value = p.next().unwrap().as_str();
    Ok((name, value))
}

fn r_meta_key(p: P, ctx: Context) -> Result<SimpleExpr> {
    let p = p.single()?;
    match p.as_rule() {
        Rule::ident => Ok(SimpleExpr::KeyVar(r_var(p, ctx)?)),
        Rule::event_key => {
            let mut p = p.inner()?;
            let lamport = p.natural()?.into();
            let stream = p.parse_or_default()?;
            Ok(SimpleExpr::KeyLiteral(SortKey { lamport, stream }))
        }
        x => bail!("unexpected token: {:?}", x),
    }
}

fn r_meta_time(p: P, ctx: Context) -> Result<SimpleExpr> {
    let p = p.single()?;
    match p.as_rule() {
        Rule::ident => Ok(SimpleExpr::TimeVar(r_var(p, ctx)?)),
        Rule::isodate => Ok(SimpleExpr::TimeLiteral(r_timestamp(p)?)),
        x => bail!("unexpected token: {:?}", x),
    }
}

fn r_sub_query(p: P, ctx: Context) -> Result<Query<'static>> {
    r_query(Vec::new(), Vec::new(), p.single()?, ctx)
}

fn r_simple_expr(p: P, ctx: Context) -> Result<SimpleExpr> {
    static PRATT: Lazy<PrattParser<Rule>> = Lazy::new(|| {
        PrattParser::new()
            .op(Op::infix(Rule::alternative, Assoc::Left))
            .op(Op::infix(Rule::or, Assoc::Left))
            .op(Op::infix(Rule::xor, Assoc::Left))
            .op(Op::infix(Rule::and, Assoc::Left))
            .op(Op::infix(Rule::eq, Assoc::Left) | Op::infix(Rule::ne, Assoc::Left))
            .op(Op::infix(Rule::lt, Assoc::Left)
                | Op::infix(Rule::le, Assoc::Left)
                | Op::infix(Rule::gt, Assoc::Left)
                | Op::infix(Rule::ge, Assoc::Left))
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left)
                | Op::infix(Rule::div, Assoc::Left)
                | Op::infix(Rule::modulo, Assoc::Left))
            .op(Op::infix(Rule::pow, Assoc::Left))
            .op(Op::prefix(Rule::not))
    });
    PRATT
        .map_primary(|p| {
            Ok(match p.as_rule() {
                Rule::decimal => SimpleExpr::Number(r_number(p)?),
                Rule::var_index => r_var_index(p, ctx)?,
                Rule::expr_index => r_expr_index(p, ctx)?,
                Rule::simple_expr => r_simple_expr(p, ctx)?,
                Rule::string => SimpleExpr::String(r_string(p)?),
                Rule::interpolation => SimpleExpr::Interpolation(r_interpolation(p, ctx)?),
                Rule::object => SimpleExpr::Object(r_object(p, ctx)?),
                Rule::array => SimpleExpr::Array(r_array(p, ctx)?),
                Rule::null => SimpleExpr::Null,
                Rule::bool => SimpleExpr::Bool(r_bool(p)),
                Rule::simple_cases => SimpleExpr::Cases(r_cases(p, ctx)?),
                Rule::aggr_op => r_aggr(p, ctx)?,
                Rule::func_call => SimpleExpr::FuncCall(r_func_call(p, ctx)?),
                Rule::sub_query => SimpleExpr::SubQuery(r_sub_query(p, ctx)?),
                Rule::meta_key => r_meta_key(p, ctx)?,
                Rule::meta_time => r_meta_time(p, ctx)?,
                Rule::meta_tags => SimpleExpr::Tags(r_var(p.single()?, ctx)?),
                Rule::meta_app => SimpleExpr::App(r_var(p.single()?, ctx)?),
                x => bail!("unexpected token: {:?}", x),
            })
        })
        .map_prefix(|op, rhs| {
            Ok(match op.as_rule() {
                Rule::not => SimpleExpr::Not(rhs?.into()),
                x => bail!("unexpected token: {:?}", x),
            })
        })
        .map_infix(|lhs, op, rhs| {
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
        })
        .parse(p.inner()?)
}

fn r_query<'a>(pragmas: Vec<(&'a str, &'a str)>, features: Vec<String>, p: P, ctx: Context) -> Result<Query<'a>> {
    let mut p = p.inner()?;
    let source = match p.peek().unwrap().as_rule() {
        Rule::tag_expr => {
            let from = r_tag_expr(p.next().ok_or(NoVal("tag expression"))?, ctx.simple())?;
            let mut order = None;
            if let Some(o) = p.peek() {
                if o.as_rule() == Rule::query_order {
                    let o = p.next().unwrap();
                    order = Some(r_order(o)?);
                }
            }
            Source::Events { from, order }
        }
        Rule::array => Source::Array(r_array(p.next().unwrap(), ctx)?),
        x => bail!("unexpected token: {:?}", x),
    };
    let mut q = Query {
        pragmas,
        features,
        source,
        ops: vec![],
    };
    for o in p {
        match o.as_rule() {
            Rule::filter => q.ops.push(Operation::Filter(r_simple_expr(o.single()?, ctx.simple())?)),
            Rule::select => {
                let v = r_array(o, ctx.simple())?.items.to_vec();
                q.ops.push(Operation::Select(v.try_into()?))
            }
            Rule::aggregate => q
                .ops
                .push(Operation::Aggregate(r_simple_expr(o.single()?, ctx.aggregate())?)),
            Rule::limit => q.ops.push(Operation::Limit(o.single()?.natural()?.try_into()?)),
            Rule::binding => {
                let mut p = o.inner()?;
                let ident = p.string()?;
                let expr = r_simple_expr(p.single()?, ctx.simple())?;
                q.ops.push(Operation::Binding(ident, expr));
            }
            x => bail!("unexpected token: {:?}", x),
        }
    }
    Ok(q)
}

pub(crate) fn query_from_str(s: &str) -> Result<Query<'_>> {
    let p = Aql::parse(Rule::main_query, s)?.single()?;
    let mut p = p.inner()?;
    let mut pragmas = Vec::new();
    while p.peek().map(|p| p.as_rule()) == Some(Rule::pragma) {
        pragmas.push(r_pragma(p.next().unwrap())?);
    }
    let mut f = p.next().ok_or(NoVal("main query"))?;
    let features = if f.as_rule() == Rule::features {
        let features = f.inner()?.map(|mut ff| ff.string()).collect::<Result<_>>()?;
        f = p.next().ok_or(NoVal("FROM"))?;
        features
    } else {
        vec![]
    };
    let now = Timestamp::now();
    r_query(pragmas, features, f, Context::Simple { now })
}

impl TryFrom<(Timestamp, &str)> for TagExpr {
    type Error = anyhow::Error;

    fn try_from((now, s): (Timestamp, &str)) -> Result<Self, Self::Error> {
        let p = Aql::parse(Rule::main_tag_expr, s)?.single()?.single()?;
        r_tag_expr(p, Context::Simple { now })
    }
}

impl FromStr for TagExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_tag_expr, s)?.single()?.single()?;
        let now = Timestamp::now();
        r_tag_expr(p, Context::Simple { now })
    }
}

impl FromStr for SimpleExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_simple_expr, s)?.single()?.single()?;
        let now = Timestamp::now();
        r_simple_expr(p, Context::Simple { now })
    }
}

impl FromStr for super::var::Var {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_ident, s)?.single()?;
        Ok(Self(p.as_str().nfc().collect()))
    }
}

pub fn is_ident(s: &str) -> bool {
    Aql::parse(Rule::main_ident, s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Var;
    use ax_types::{tag, NodeId, StreamId};
    use pest::{fails_with, Parser};
    use std::convert::TryFrom;

    fn s(s: &str) -> SimpleExpr {
        SimpleExpr::String(s.to_owned())
    }
    fn o(q: Query) -> Option<Order> {
        match q.source {
            Source::Events { order, .. } => order,
            Source::Array(_) => panic!("expected Source::Events"),
        }
    }
    fn from(q: Query) -> TagExpr {
        match q.source {
            Source::Events { from, .. } => from,
            Source::Array(_) => panic!("expected Source::Events"),
        }
    }
    fn arr(q: Query) -> Vec<String> {
        match q.source {
            Source::Events { .. } => panic!("expected Source::Array"),
            Source::Array(Arr { items }) => items.iter().map(|x| x.to_string()).collect(),
        }
    }

    #[test]
    fn tag() -> Result<()> {
        let p = Aql::parse(Rule::tag, "'hello''s revenge'")?;
        assert_eq!(
            r_tag(p.single()?, Context::Simple { now: Timestamp::now() })?,
            TagExpr::Atom(TagAtom::Tag(tag!("hello's revenge")))
        );
        let p = Aql::parse(Rule::tag, "\"hello\"\"s revenge\"")?;
        assert_eq!(
            r_tag(p.single()?, Context::Simple { now: Timestamp::now() })?,
            TagExpr::Atom(TagAtom::Tag(tag!("hello\"s revenge")))
        );
        Ok(())
    }

    #[test]
    fn tag_expr() -> Result<()> {
        use TagAtom::{FromLamport, FromTime, Tag, ToLamport, ToTime};
        use TagExpr::*;
        assert_eq!(
            "'x' |\t'y'\n&'z'".parse::<TagExpr>()?,
            Or((
                Atom(Tag(tag!("x"))),
                And((Atom(Tag(tag!("y"))), Atom(Tag(tag!("z")))).into())
            )
                .into())
        );
        assert_eq!(
            "'a' & TIME > 1986-12-15Z & TIME < 2001-03-27+04:00".parse::<TagExpr>()?,
            And((
                And((Atom(Tag(tag!("a"))), Atom(FromTime(534988800000000.into(), false)),).into()),
                Atom(ToTime(985636800000000.into(), false))
            )
                .into())
        );
        let stream = NodeId::new([
            12, 65, 70, 28, 130, 74, 44, 32, 196, 20, 97, 200, 36, 162, 194, 12, 65, 70, 28, 130, 74, 44, 32, 196, 20,
            97, 200, 36, 162, 194, 12, 65,
        ])
        .stream(4312.into());
        assert_eq!(
            "'a' & KEY >= 12/1234567890123456789012345678901234567890122-4312 & \
             KEY <= 13/1234567890123456789012345678901234567890122-4312"
                .parse::<TagExpr>()?,
            And((
                And((
                    Atom(Tag(tag!("a"))),
                    Atom(FromLamport(
                        SortKey {
                            lamport: 12.into(),
                            stream
                        },
                        true
                    )),
                )
                    .into()),
                Atom(ToLamport(
                    SortKey {
                        lamport: 13.into(),
                        stream
                    },
                    true
                ))
            )
                .into())
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 7s ago")).unwrap(),
            Atom(FromTime(Timestamp::new(99_999_993_000_000), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 2m ago")).unwrap(),
            Atom(FromTime(Timestamp::new(99_999_880_000_000), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 2h ago")).unwrap(),
            Atom(FromTime(Timestamp::new(99_992_800_000_000), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 1D ago")).unwrap(),
            Atom(FromTime(Timestamp::new(99_913_600_000_000), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 1W ago")).unwrap(),
            Atom(FromTime(Timestamp::new(99_395_200_000_000), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 1M ago")).unwrap(),
            Atom(FromTime(Timestamp::new(97_448_557_123_092), false))
        );
        assert_eq!(
            TagExpr::try_from((Timestamp::new(100_000_000_000_000), "TIME > 1Y ago")).unwrap(),
            Atom(FromTime(Timestamp::new(68_443_074_749_267), false))
        );

        Ok(())
    }

    #[test]
    fn order() -> Result<()> {
        let q = Query::parse("FROM 'x'").unwrap();
        assert_eq!(o(q), None);
        let q = Query::parse("FROM 'x' ORDER ASC").unwrap();
        assert_eq!(o(q), Some(Order::Asc));
        let q = Query::parse("FROM 'x' ORDER DESC").unwrap();
        assert_eq!(o(q), Some(Order::Desc));
        let q = Query::parse("FROM 'x' ORDER STREAM").unwrap();
        assert_eq!(o(q), Some(Order::StreamAsc));
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
        use super::{Arr, Ind, Num::*, Obj};
        use ax_types::app_id;
        use SimpleExpr::*;
        use TagAtom::*;
        use TagExpr::Atom;

        assert_eq!(
            Query::parse("FROM 'machine' | 'user' END")?,
            Query::new(Tag(tag!("machine")).or(Tag(tag!("user"))))
        );
        assert_eq!(
            Query::parse(
                "FROM 'machine' |
                -- or the other
                  'user' & isLocal & from(2012-12-31Z) & to(12345678901234567) & \
                  from(10/1234567890123456789012345678901234567890122-4312) & appId(hello-5.-x-) & allEvents
                  FILTER _.x[42] > 5 SELECT { x: ! 'hello' y: 42 z: [1.3,_.x] } END --"
            )?,
            Query::new(
                Atom(Tag(tag!("machine"))).or(Tag(tag!("user"))
                    .and(IsLocal)
                    .and(Atom(FromTime(1356912000000000.into(), true)))
                    .and(Atom(ToLamport(
                        SortKey {
                            lamport: 12345678901234567.into(),
                            stream: StreamId::min(),
                        },
                        false
                    )))
                    .and(Atom(FromLamport(
                        SortKey {
                            lamport: 10.into(),
                            stream: NodeId::new([
                                12, 65, 70, 28, 130, 74, 44, 32, 196, 20, 97, 200, 36, 162, 194, 12, 65, 70, 28, 130,
                                74, 44, 32, 196, 20, 97, 200, 36, 162, 194, 12, 65
                            ])
                            .stream(4312.into())
                        },
                        true
                    )))
                    .and(Atom(AppId(app_id!("hello-5.-x-"))))
                    .and(Atom(AllEvents)))
            )
            .with_op(Operation::Filter(Ind::with("_", &[&"x", &42]).gt(Number(Natural(5)))))
            .with_op(Operation::Select(
                vec![Obj::with(&[
                    ("x", Not(String("hello".to_owned()).into())),
                    ("y", Number(Natural(42))),
                    ("z", Arr::with(&[Number(Decimal(1.3)), Ind::with("_", &[&"x"])]))
                ])
                .with_spread(false)]
                .try_into()?
            ))
        );
        Ok(())
    }

    #[test]
    fn positive() {
        let p = |str: &'static str| Query::parse(str).unwrap();
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
            positives: vec![Rule::array, Rule::tag_expr],
            negatives: vec![],
            pos: 5
        };
        fails_with! {
            parser: Aql,
            input: "FROM 'x' ELECT 'x'",
            rule: Rule::main_query,
            positives: vec![Rule::EOI, Rule::query_order, Rule::filter, Rule::select, Rule::aggregate, Rule::limit, Rule::binding, Rule::and, Rule::or],
            negatives: vec![],
            pos: 9
        };
        fails_with! {
            parser: Aql,
            input: "FROM 'x' FITTER 'x'",
            rule: Rule::main_query,
            positives: vec![Rule::EOI, Rule::query_order, Rule::filter, Rule::select, Rule::aggregate, Rule::limit, Rule::binding, Rule::and, Rule::or],
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
                items: vec![Number(Natural(1)).with_spread(false), Bool(true).with_spread(false)].into()
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
            let q = Query::parse(s);
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
            .with_spread(false)
        };
        let s = |s: &str| Index::String(s.to_owned());
        let n = |n: u64| SimpleExpr::Number(Num::Natural(n));
        let v = |s: &str| SimpleExpr::Variable(Var::try_from(s).unwrap()).with_spread(false);

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
            })
            .with_spread(false)])
        )
    }

    #[test]
    fn index() {
        let p = |s: &str| s.parse::<SimpleExpr>().unwrap();
        assert_eq!(
            p("a['ª']"),
            SimpleExpr::Indexing(Ind {
                head: Arc::new(SimpleExpr::Variable(Var::try_from("a").unwrap())),
                tail: vec![Index::String("ª".to_owned())].try_into().unwrap()
            })
        );
        assert_eq!(
            p("a.ª"),
            SimpleExpr::Indexing(Ind {
                head: Arc::new(SimpleExpr::Variable(Var::try_from("a").unwrap())),
                tail: vec![Index::String("ª".to_owned())].try_into().unwrap()
            })
        );
        assert_eq!(p("a['ª']").to_string(), "a.ª");

        assert_eq!(
            p("{ⓐ:1}"),
            SimpleExpr::Object(Obj {
                props: vec![(Index::String("ⓐ".to_owned()), SimpleExpr::Number(Num::Natural(1)))].into()
            })
        );
        assert_eq!(p("{ⓐ:1}").to_string(), "{ ⓐ: 1 }");
    }

    #[test]
    fn aggregate() {
        let p = |s: &str, e: Option<&str>| {
            let q = Query::parse(s);
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

    #[test]
    fn limit() {
        let q = Query::parse("FROM 'x' LIMIT 10").unwrap();
        assert_eq!(&q.ops[0], &Operation::Limit(10.try_into().unwrap()));
    }

    #[test]
    fn func_call() {
        let p = |s: &str, e: Option<&str>| {
            let q = Query::parse(s);
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

        assert_eq!(
            p("FROM 'x' SELECT Func()", None),
            Some(vec![SimpleExpr::FuncCall(FuncCall {
                name: "Func".to_owned(),
                args: vec![].into()
            })
            .with_spread(false)])
        );
        assert_eq!(
            p("FROM 'x' SELECT Fÿnc('x')", None),
            Some(vec![SimpleExpr::FuncCall(FuncCall {
                name: "Fÿnc".to_owned(),
                args: vec![SimpleExpr::String("x".to_owned())].into()
            })
            .with_spread(false)])
        );
        assert_eq!(
            p("FROM 'x' SELECT Func(x, 'x')", None),
            Some(vec![SimpleExpr::FuncCall(FuncCall {
                name: "Func".to_owned(),
                args: vec![
                    SimpleExpr::Variable(Var::try_from("x").unwrap()),
                    SimpleExpr::String("x".to_owned())
                ]
                .into()
            })
            .with_spread(false)])
        );
    }

    #[test]
    fn interpolation() {
        use TagAtom::*;
        use TagExpr::*;
        fn arr(v: Vec<SimpleExpr>) -> Arr<SimpleExpr> {
            Arr { items: v.into() }
        }

        let q = Query::parse("FROM `a{U+1e}b`").unwrap();
        assert_eq!(from(q), Atom(Interpolation(arr(vec![s("a\x1eb")]))));

        let q = Query::parse("FROM `a{U+1e}`").unwrap();
        assert_eq!(from(q), Atom(Interpolation(arr(vec![s("a\x1e")]))));

        let q = Query::parse("FROM `{U+1e}b`").unwrap();
        assert_eq!(from(q), Atom(Interpolation(arr(vec![s("\x1eb")]))));

        let q = Query::parse("FROM `a{U+1e}b{U+1f}`").unwrap();
        assert_eq!(from(q), Atom(Interpolation(arr(vec![s("a\x1eb\x1f")]))));

        let q = Query::parse("FROM `a{U+1e}{U+1f}b`").unwrap();
        assert_eq!(from(q), Atom(Interpolation(arr(vec![s("a\x1e\x1fb")]))));

        let q = Query::parse("FROM `a{U+110000}`").unwrap_err();
        assert_eq!(q.to_string(), "invalid unicode scalar value `U+110000`");
    }

    #[test]
    fn from_arr() {
        let q = Query::parse("FROM [1, 2, 3]").unwrap();
        assert_eq!(arr(q), vec!["1", "2", "3"]);
    }

    #[test]
    fn spread() {
        let q = Query::parse("FROM [1, ...2, ...(x + 3)]").unwrap();
        assert_eq!(arr(q), vec!["1", "...2", "...(x + 3)"]);

        let q = Query::parse("FROM 'x' SELECT ...a, b, ...c").unwrap();
        let v = match q.ops.into_iter().next() {
            Some(Operation::Select(v)) => v.iter().map(|e| e.spread).collect::<Vec<_>>(),
            x => panic!("unexpected: {:?}", x),
        };
        assert_eq!(v, vec![true, false, true]);
    }

    #[test]
    fn timestamp() {
        let t = |t| r_timestamp(Aql::parse(Rule::isodate, t).unwrap().single().unwrap()).unwrap();

        assert_eq!(t("2022-01-02Z"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02+00:00"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02+01:00"), Timestamp::new(1641078000000000));
        assert_eq!(t("2022-01-02T00:00Z"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02T00:00+00:00"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02T00:00+01:00"), Timestamp::new(1641078000000000));
        assert_eq!(t("2022-01-02T00:00:00Z"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02T00:00:00+00:00"), Timestamp::new(1641081600000000));
        assert_eq!(t("2022-01-02T00:00:00+01:00"), Timestamp::new(1641078000000000));
        assert_eq!(t("2022-01-02T00:00:00.001Z"), Timestamp::new(1641081600001000));
        assert_eq!(t("2022-01-02T00:00:00.001+00:00"), Timestamp::new(1641081600001000));
        assert_eq!(t("2022-01-02T00:00:00.001+01:00"), Timestamp::new(1641078000001000));
        assert_eq!(t("2022-01-02T00:00:00.000002Z"), Timestamp::new(1641081600000002));
        assert_eq!(t("2022-01-02T00:00:00.000002+00:00"), Timestamp::new(1641081600000002));
        assert_eq!(t("2022-01-02T00:00:00.000002-01:00"), Timestamp::new(1641085200000002));
        assert_eq!(t("2022-01-02T00:00:00.000003000Z"), Timestamp::new(1641081600000003));
        assert_eq!(
            t("2022-01-02T00:00:00.000003000+00:00"),
            Timestamp::new(1641081600000003)
        );
        assert_eq!(
            t("2022-01-02T00:00:00.000003000+01:00"),
            Timestamp::new(1641078000000003)
        );
    }

    #[test]
    fn pragma() {
        let s = "PRAGMA x := y \nFROM 'x'".to_owned();
        let q = Query::parse(&s).unwrap();
        assert_eq!(q.pragmas, vec![("x", "y ")]);
        assert!(std::ptr::eq(q.pragmas[0].0, &s[7..8]));
        assert!(std::ptr::eq(q.pragmas[0].1, &s[12..14]));

        let s = "PRAGMA x \ny \nENDPRAGMA\nFROM 'x'".to_owned();
        let q = Query::parse(&s).unwrap();
        assert_eq!(q.pragmas, vec![("x", "y ")]);
        assert!(std::ptr::eq(q.pragmas[0].0, &s[7..8]));
        assert!(std::ptr::eq(q.pragmas[0].1, &s[10..12]));

        let s = "PRAGMA x \ny \nENDPRAGMA\n
            PRAGMA a :=hello
            FROM 'x'"
            .to_owned();
        let q = Query::parse(&s).unwrap();
        assert_eq!(q.pragmas, vec![("x", "y "), ("a", "hello")]);
    }
}
