#![allow(dead_code)]
use super::{Array, Expression, Index, Number, Object, Operation, Path, Query, SimpleExpr, TagExpr};
use crate::{tagged::Tag, TimeStamp};
use chrono::{TimeZone, Utc};
use once_cell::sync::Lazy;
use pest::{prec_climber::PrecClimber, Parser};
use utils::*;

#[derive(pest_derive::Parser)]
#[grammar = "language/aql.pest"]
struct AQL;

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
        Rule::single_quoted => Tag::new(s.replace("''", "'")).unwrap(),
        Rule::double_quoted => Tag::new(s.replace("\"\"", "\"")).unwrap(),
        x => unexpected!(x),
    }
}

enum FromTo {
    From,
    To,
}
fn r_tag_from_to(mut p: P, f: FromTo) -> TagExpr {
    match p.rule() {
        Rule::natural => match f {
            FromTo::From => TagExpr::FromLamport(p.natural().into()),
            FromTo::To => TagExpr::ToLamport(p.natural().into()),
        },
        Rule::isodate => match f {
            FromTo::From => TagExpr::FromTime(r_timestamp(p)),
            FromTo::To => TagExpr::ToTime(r_timestamp(p)),
        },
        x => unexpected!(x),
    }
}

fn r_tag_expr(p: P) -> TagExpr {
    static CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
        use pest::prec_climber::{Assoc::*, Operator};

        PrecClimber::new(vec![Operator::new(Rule::or, Left), Operator::new(Rule::and, Left)])
    });

    CLIMBER.climb(
        p.inner(),
        |p| match p.rule() {
            Rule::tag => TagExpr::Tag(r_tag(p)),
            Rule::tag_expr => r_tag_expr(p),
            Rule::all_events => TagExpr::AllEvents,
            Rule::is_local => TagExpr::IsLocal,
            Rule::tag_from => r_tag_from_to(p.single(), FromTo::From),
            Rule::tag_to => r_tag_from_to(p.single(), FromTo::To),
            Rule::tag_app => TagExpr::AppId(p.single().as_str().parse().unwrap()),
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

fn r_timestamp(p: P) -> TimeStamp {
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

pub fn expression(input: &str) -> R<Expression> {
    let p = AQL::parse(Rule::expression, input)?.single().single();
    match p.rule() {
        Rule::simple_expr => Ok(Expression::Simple(r_simple_expr(p))),
        Rule::query => Ok(Expression::Query(r_query(p))),
        x => unexpected!(x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag;
    use pest::{fails_with, Parser};

    #[test]
    fn tag() {
        let p = AQL::parse(Rule::tag, "'hello''s revenge'").unwrap();
        assert_eq!(r_tag(p.single()), tag!("hello's revenge"));
        let p = AQL::parse(Rule::tag, "\"hello\"\"s revenge\"").unwrap();
        assert_eq!(r_tag(p.single()), tag!("hello\"s revenge"));
    }

    #[test]
    fn tag_expr() {
        use TagExpr::*;
        let p = AQL::parse(Rule::tag_expr, "'x' |\t'y'\n&'z'").unwrap();
        assert_eq!(
            r_tag_expr(p.single()),
            Or((Tag(tag!("x")), And((Tag(tag!("y")), Tag(tag!("z"))).into())).into())
        );
    }

    #[test]
    fn simple_expr() {
        use super::Number::*;
        use super::Path;
        use SimpleExpr::*;
        let p = AQL::parse(Rule::simple_expr, "(x - 5.2 * 1234)^2 / 7 % 5").unwrap();
        assert_eq!(
            r_simple_expr(p.single()),
            Path::ident("x")
                .sub(Number(Decimal(5.2)).mul(Number(Natural(1234))))
                .pow(Number(Natural(2)))
                .div(Number(Natural(7)))
                .modulo(Number(Natural(5)))
        )
    }

    #[test]
    fn expr() {
        use super::Number::*;
        use super::{Array, Object, Path};
        use crate::app_id;
        use SimpleExpr::*;
        use TagExpr::*;
        assert_eq!(
            expression("FROM 'machine' | 'user' END").unwrap(),
            Expression::Query(Query::new(Tag(tag!("machine")).or(Tag(tag!("user")))))
        );
        assert_eq!(
            expression(
                "FROM 'machine' |
                -- or the other
                  'user' & isLocal & from(2012-12-31) & to(12345678901234567) & appId( hello-5._x_ ) & allEvents
                  FILTER _.x.42 > 5 SELECT { x: ! 'hello' y: 42 z: [1.3,_.x] } END --"
            )
            .unwrap(),
            Expression::Query(
                Query::new(
                    Tag(tag!("machine")).or(Tag(tag!("user"))
                        .and(IsLocal)
                        .and(FromTime(1356912000000000.into()))
                        .and(ToLamport(12345678901234567.into()))
                        .and(AppId(app_id!("hello-5._x_")))
                        .and(AllEvents))
                )
                .with_op(Operation::Filter(Path::with("_", &[&"x", &42]).gt(Number(Natural(5)))))
                .with_op(Operation::Select(Object::with(&[
                    ("x", Not(String("hello".to_owned()).into())),
                    ("y", Number(Natural(42))),
                    ("z", Array::with(&[Number(Decimal(1.3)), Path::with("_", &[&"x"])]))
                ])))
            )
        );
    }

    #[test]
    fn negative() {
        fails_with! {
            parser: AQL,
            input: "FROM x",
            rule: Rule::expression,
            positives: vec![Rule::tag_expr],
            negatives: vec![],
            pos: 5
        };
    }
}
