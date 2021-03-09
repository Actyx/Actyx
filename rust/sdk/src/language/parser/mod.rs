#![allow(dead_code)]
use super::{Expression, Operation, Query, SimpleExpr, TagExpr};
use crate::tagged::Tag;
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
            Rule::number => SimpleExpr::Number(p.as_str().parse().unwrap()),
            Rule::ident => SimpleExpr::Ident(p.as_str().to_owned()),
            Rule::simple_expr => r_simple_expr(p),
            Rule::simple_not => SimpleExpr::Not(primary(p.single()).into()),
            Rule::string => SimpleExpr::String(r_string(p)),
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
    let mut q = Query::new(r_tag_expr(p.next().unwrap()));
    for o in p {
        match o.rule() {
            Rule::filter => q.push(Operation::Filter(r_simple_expr(o.single()))),
            Rule::select => q.push(Operation::Select(r_simple_expr(o.single()))),
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
        use SimpleExpr::*;
        let p = AQL::parse(Rule::simple_expr, "(x - 5.2 * 1234)^2 / 7 % 5").unwrap();
        assert_eq!(
            r_simple_expr(p.single()),
            Ident("x".to_owned())
                .sub(Number(5.2).mul(Number(1234.0)))
                .pow(Number(2.0))
                .div(Number(7.0))
                .modulo(Number(5.0))
        )
    }

    #[test]
    fn expr() {
        use SimpleExpr::*;
        use TagExpr::Tag;
        assert_eq!(
            expression("FROM 'machine' | 'user' END").unwrap(),
            Expression::Query(Query::new(Tag(tag!("machine")).or(Tag(tag!("user")))))
        );
        assert_eq!(
            expression("FROM 'machine' | 'user' FILTER _ > 5 SELECT ! 'hello' END").unwrap(),
            Expression::Query(
                Query::new(Tag(tag!("machine")).or(Tag(tag!("user"))))
                    .with_op(Operation::Filter(Ident("_".to_owned()).gt(Number(5.0))))
                    .with_op(Operation::Select(Not(String("hello".to_owned()).into())))
            )
        );
    }

    #[test]
    fn negative() {
        fails_with! {
            parser: AQL,
            input: "FROM x",
            rule: Rule::expression,
            positives: vec![Rule::nonempty_string],
            negatives: vec![],
            pos: 5
        };
    }
}
