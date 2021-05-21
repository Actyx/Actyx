#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]

use std::str::FromStr;

use super::{Array, Index, Number, Object, Operation, Path, Query, SimpleExpr, TagAtom, TagExpr};
use crate::{tags::Tag, Timestamp};
use chrono::{TimeZone, Utc};
use once_cell::sync::Lazy;
use pest::{prec_climber::PrecClimber, Parser};
use utils::*;

#[derive(pest_derive::Parser)]
#[grammar = "language/aql.pest"]
struct Aql;

mod utils;

macro_rules! unexpected {
    ($x:ident) => {{
        panic!("unexpected {:?}", $x)
    }};
}

fn r_tag(p: P) -> Tag {
    let quoted = p.single().single();
    let s = quoted.as_str();
    let s = &s[1..s.len() - 1];
    match quoted.as_rule() {
        Rule::single_quoted => Tag::from_str(s.replace("''", "'").as_ref()).unwrap(),
        Rule::double_quoted => Tag::from_str(s.replace("\"\"", "\"").as_ref()).unwrap(),
        x => unexpected!(x),
    }
}

enum FromTo {
    From,
    To,
}
fn r_tag_from_to(mut p: P, f: FromTo) -> TagExpr {
    use TagAtom::*;
    use TagExpr::Atom;
    match p.rule() {
        Rule::natural => match f {
            FromTo::From => Atom(FromLamport(p.natural().into())),
            FromTo::To => Atom(ToLamport(p.natural().into())),
        },
        Rule::isodate => match f {
            FromTo::From => Atom(FromTime(r_timestamp(p))),
            FromTo::To => Atom(ToTime(r_timestamp(p))),
        },
        x => unexpected!(x),
    }
}

fn r_tag_expr(p: P) -> TagExpr {
    use TagAtom::*;
    use TagExpr::Atom;

    static CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
        use pest::prec_climber::{Assoc::*, Operator};

        PrecClimber::new(vec![Operator::new(Rule::or, Left), Operator::new(Rule::and, Left)])
    });

    CLIMBER.climb(
        p.inner(),
        |p| match p.rule() {
            Rule::tag => Atom(Tag(r_tag(p))),
            Rule::tag_expr => r_tag_expr(p),
            Rule::all_events => Atom(AllEvents),
            Rule::is_local => Atom(IsLocal),
            Rule::tag_from => r_tag_from_to(p.single(), FromTo::From),
            Rule::tag_to => r_tag_from_to(p.single(), FromTo::To),
            Rule::tag_app => Atom(AppId(p.single().as_str().parse().unwrap())),
            x => unexpected!(x),
        },
        |lhs, op, rhs| match op.rule() {
            Rule::and => lhs.and(rhs),
            Rule::or => lhs.or(rhs),
            x => unexpected!(x),
        },
    )
}

fn r_string(p: P) -> String {
    let p = p.single();
    match p.rule() {
        Rule::nonempty_string => {
            let p = p.single();
            let s = p.as_str();
            let s = &s[1..s.len() - 1];
            match p.rule() {
                Rule::single_quoted => s.replace("''", "'"),
                Rule::double_quoted => s.replace("\"\"", "\""),
                x => unexpected!(x),
            }
        }
        Rule::empty_string => String::new(),
        x => unexpected!(x),
    }
}

fn r_path(p: P) -> Path {
    let mut p = p.inner();
    let mut ret = Path {
        head: p.string(),
        tail: vec![],
    };
    for mut i in p {
        match i.rule() {
            Rule::ident => ret.tail.push(Index::Ident(i.string())),
            Rule::natural => ret.tail.push(Index::Number(i.natural())),
            x => unexpected!(x),
        }
    }
    ret
}

fn r_number(p: P) -> Number {
    let mut p = p.single();
    match p.rule() {
        Rule::decimal => Number::Decimal(p.decimal()),
        Rule::natural => Number::Natural(p.natural()),
        x => unexpected!(x),
    }
}

fn r_timestamp(p: P) -> Timestamp {
    let mut p = p.inner();
    let year: i32 = p.string().parse().unwrap();
    let month: u32 = p.string().parse().unwrap();
    let day: u32 = p.string().parse().unwrap();
    let hour: u32 = p.parse_or_default();
    let min: u32 = p.parse_or_default();
    let sec: u32 = p.parse_or_default();
    let nano: u32 = if let Some(p) = p.next() {
        match p.rule() {
            Rule::millisecond => p.as_str().parse::<u32>().unwrap() * 1_000_000,
            Rule::microsecond => p.as_str().parse::<u32>().unwrap() * 1_000,
            Rule::nanosecond => p.as_str().parse::<u32>().unwrap(),
            x => unexpected!(x),
        }
    } else {
        0
    };
    Utc.ymd(year, month, day).and_hms_nano(hour, min, sec, nano).into()
}

fn r_object(p: P) -> Object {
    let mut v = vec![];
    let mut p = p.inner();
    while p.peek().is_some() {
        v.push((p.string(), r_simple_expr(p.next().unwrap())));
    }
    Object { props: v }
}

fn r_array(p: P) -> Array {
    Array {
        items: p.inner().map(r_simple_expr).collect(),
    }
}

fn r_simple_expr(p: P) -> SimpleExpr {
    static CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
        use pest::prec_climber::{Assoc::*, Operator};
        let op = Operator::new;

        PrecClimber::new(vec![
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

    fn primary(p: P) -> SimpleExpr {
        match p.rule() {
            Rule::number => SimpleExpr::Number(r_number(p)),
            Rule::path => SimpleExpr::Path(r_path(p)),
            Rule::simple_expr => r_simple_expr(p),
            Rule::simple_not => SimpleExpr::Not(primary(p.single()).into()),
            Rule::string => SimpleExpr::String(r_string(p)),
            Rule::object => SimpleExpr::Object(r_object(p)),
            Rule::array => SimpleExpr::Array(r_array(p)),
            x => unexpected!(x),
        }
    }

    CLIMBER.climb(p.inner(), primary, |lhs, op, rhs| match op.rule() {
        Rule::add => lhs.add(rhs),
        Rule::sub => lhs.sub(rhs),
        Rule::mul => lhs.mul(rhs),
        Rule::div => lhs.div(rhs),
        Rule::modulo => lhs.modulo(rhs),
        Rule::pow => lhs.pow(rhs),
        Rule::and => lhs.and(rhs),
        Rule::or => lhs.or(rhs),
        Rule::xor => lhs.xor(rhs),
        Rule::lt => lhs.lt(rhs),
        Rule::le => lhs.le(rhs),
        Rule::gt => lhs.gt(rhs),
        Rule::ge => lhs.ge(rhs),
        Rule::eq => lhs.eq(rhs),
        Rule::ne => lhs.ne(rhs),
        x => unexpected!(x),
    })
}

fn r_query(p: P) -> Query {
    let mut p = p.inner();
    let mut q = Query {
        from: r_tag_expr(p.next().unwrap()),
        ops: vec![],
    };
    for o in p {
        match o.rule() {
            Rule::filter => q.ops.push(Operation::Filter(r_simple_expr(o.single()))),
            Rule::select => q.ops.push(Operation::Select(r_simple_expr(o.single()))),
            x => unexpected!(x),
        }
    }
    q
}

impl FromStr for Query {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_query, s)?.single().single();
        Ok(r_query(p))
    }
}

impl FromStr for TagExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_tag_expr, s)?.single().single();
        Ok(r_tag_expr(p))
    }
}

impl FromStr for SimpleExpr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Aql::parse(Rule::main_simple_expr, s)?.single().single();
        Ok(r_simple_expr(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag;
    use pest::{fails_with, Parser};

    #[test]
    fn tag() {
        let p = Aql::parse(Rule::tag, "'hello''s revenge'").unwrap();
        assert_eq!(r_tag(p.single()), tag!("hello's revenge"));
        let p = Aql::parse(Rule::tag, "\"hello\"\"s revenge\"").unwrap();
        assert_eq!(r_tag(p.single()), tag!("hello\"s revenge"));
    }

    #[test]
    fn tag_expr() {
        use TagAtom::Tag;
        use TagExpr::*;
        assert_eq!(
            "'x' |\t'y'\n&'z'".parse::<TagExpr>().unwrap(),
            Or((
                Atom(Tag(tag!("x"))),
                And((Atom(Tag(tag!("y"))), Atom(Tag(tag!("z")))).into())
            )
                .into())
        );
    }

    #[test]
    fn simple_expr() {
        use super::Number::*;
        use super::Path;
        use SimpleExpr::*;
        assert_eq!(
            "(x - 5.2 * 1234)^2 / 7 % 5".parse::<SimpleExpr>().unwrap(),
            Path::ident("x")
                .sub(Number(Decimal(5.2)).mul(Number(Natural(1234))))
                .pow(Number(Natural(2)))
                .div(Number(Natural(7)))
                .modulo(Number(Natural(5)))
        );

        fails_with! {
            parser: Aql,
            input: "5+3!",
            rule: Rule::main_simple_expr,
            positives: vec![Rule::EOI, Rule::add, Rule::sub, Rule::mul, Rule::div, Rule::modulo, Rule::pow, Rule::and, Rule::or, Rule::xor, Rule::lt, Rule::le, Rule::gt, Rule::ge, Rule::eq, Rule::ne],
            negatives: vec![],
            pos: 3
        };
    }

    #[test]
    fn query() {
        use super::Number::*;
        use super::{Array, Object, Path};
        use crate::app_id;
        use SimpleExpr::*;
        use TagAtom::*;
        use TagExpr::Atom;

        assert_eq!(
            "FROM 'machine' | 'user' END".parse::<Query>().unwrap(),
            Query::new(Tag(tag!("machine")).or(Tag(tag!("user"))))
        );
        assert_eq!(
            "FROM 'machine' |
                -- or the other
                  'user' & isLocal & from(2012-12-31) & to(12345678901234567) & appId( hello-5._x_ ) & allEvents
                  FILTER _.x.42 > 5 SELECT { x: ! 'hello' y: 42 z: [1.3,_.x] } END --"
                .parse::<Query>()
                .unwrap(),
            Query::new(
                Atom(Tag(tag!("machine"))).or(Tag(tag!("user"))
                    .and(IsLocal)
                    .and(Atom(FromTime(1356912000000000.into())))
                    .and(Atom(ToLamport(12345678901234567.into())))
                    .and(Atom(AppId(app_id!("hello-5._x_"))))
                    .and(Atom(AllEvents)))
            )
            .with_op(Operation::Filter(Path::with("_", &[&"x", &42]).gt(Number(Natural(5)))))
            .with_op(Operation::Select(Object::with(&[
                ("x", Not(String("hello".to_owned()).into())),
                ("y", Number(Natural(42))),
                ("z", Array::with(&[Number(Decimal(1.3)), Path::with("_", &[&"x"])]))
            ])))
        );
    }

    #[test]
    fn roundtrips() {
        let rt = |str: &'static str| {
            let e = str.parse::<Query>().unwrap();
            let mut buf = String::new();
            crate::language::render::render_query(&mut buf, &e).unwrap();
            assert_eq!(buf.as_str(), str);
        };
        rt("FROM 'machine' | 'user' & isLocal & from(2012-12-31) & to(12345678901234567) & appId(hello-5._x_) & allEvents FILTER _.x.42 > 5 SELECT { x: !'hello', y: 42, z: [1.3, _.x] } END");
        rt("FROM from(2012-12-31T09:30:32.007Z) END");
        rt("FROM from(2012-12-31T09:30:32Z) END");
        rt("FROM from(2012-12-31T09:30:32.007008Z) END");
        rt("FROM 'hello''s revenge' END");
        rt("FROM 'hell''o' FILTER _.x = 'worl''d' END");
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
            positives: vec![Rule::EOI, Rule::filter, Rule::select, Rule::and, Rule::or],
            negatives: vec![],
            pos: 9
        };
        fails_with! {
            parser: Aql,
            input: "FROM 'x' FITTER 'x'",
            rule: Rule::main_query,
            positives: vec![Rule::EOI, Rule::filter, Rule::select, Rule::and, Rule::or],
            negatives: vec![],
            pos: 9
        };
    }
}
