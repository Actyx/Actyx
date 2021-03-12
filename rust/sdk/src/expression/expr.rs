use std::ops::{BitAnd, BitOr};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, none_of},
    combinator::{cut, map, value},
    error::{context, convert_error, ErrorKind, ParseError, VerboseError},
    multi::separated_list1,
    sequence::{pair, preceded, terminated},
    Err::{Error, Failure, Incomplete},
    FindToken, IResult, InputTakeAtPosition,
};
use reduce::Reduce;
use serde::{Deserialize, Deserializer};

use super::Dnf;

fn space(i: &str) -> IResult<&str, (), VerboseError<&str>> {
    let (i, _) = i.split_at_position_complete(|c| !" \r\n\t".find_token(c))?;
    Ok((i, ()))
}

fn quote(i: &str) -> IResult<&str, char, VerboseError<&str>> {
    value('\'', tag("''"))(i)
}

fn nonempty_string(i0: &str) -> IResult<&str, String, VerboseError<&str>> {
    let mut res = String::new();
    let mut i = i0;
    while let Ok((next, c)) = alt((none_of("'"), quote))(i) {
        res.push(c);
        i = next;
    }
    if res.is_empty() {
        Err(Failure(VerboseError::from_error_kind(i0, ErrorKind::NonEmpty)))
    } else {
        Ok((i, res))
    }
}

fn literal(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    context(
        "literal",
        preceded(
            pair(space, char('\'')),
            cut(terminated(map(nonempty_string, Expression::literal), char('\''))),
        ),
    )(i)
}

fn and(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    context(
        "and",
        map(separated_list1(pair(space, char('&')), alt((parens, literal))), |v| {
            if v.len() > 1 {
                Expression::and(v)
            } else {
                v.into_iter().next().unwrap()
            }
        }),
    )(i)
}

fn or(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    context(
        "or",
        map(separated_list1(pair(space, char('|')), and), |v| {
            if v.len() > 1 {
                Expression::or(v)
            } else {
                v.into_iter().next().unwrap()
            }
        }),
    )(i)
}

fn parens(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    context(
        "parens",
        preceded(pair(space, char('(')), cut(terminated(or, pair(space, char(')'))))),
    )(i)
}

/// a boolean expression, consisting of literals, union and intersection.
///
/// no attempt of simplification is made, except flattening identical operators.
///
/// `And([And([a,b]),c])` will be flattened to `And([a,b,c])`.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Expression {
    Literal(String),
    And(Vec<Expression>),
    Or(Vec<Expression>),
}

/// prints the expression with a minimum of brackets
impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        fn child_to_string(x: &Expression) -> String {
            match x {
                Expression::Or(v) if v.len() > 1 => format!("( {} )", x),
                _ => x.to_string(),
            }
        }
        write!(
            f,
            "{}",
            match self {
                Expression::Literal(text) => format!("'{}'", text.replace("'", "''")),
                Expression::And(es) => es.iter().map(child_to_string).collect::<Vec<_>>().join(" & "),
                Expression::Or(es) => es.iter().map(Expression::to_string).collect::<Vec<_>>().join(" | "),
            }
        )
    }
}

impl serde::ser::Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Parse tag expression from a string
///
/// The syntax consists of the operators & and |, parentheses ( and ), and string
/// literals delimited with single quotes ' (use '' inside a string to include a ')
impl std::str::FromStr for Expression {
    type Err = anyhow::Error;

    fn from_str(i: &str) -> Result<Self, Self::Err> {
        use anyhow::anyhow;
        match terminated(or, space)(i) {
            Ok((i, expr)) => {
                if i.is_empty() {
                    Ok(expr)
                } else {
                    Err(anyhow!("trailing garbage: '{}'", i))
                }
            }
            Err(Error(e)) | Err(Failure(e)) => Err(anyhow!(convert_error(i, e))),
            Err(Incomplete(_)) => Err(anyhow!("cannot happen")),
        }
    }
}

impl From<Dnf> for Expression {
    fn from(value: Dnf) -> Self {
        value.expression()
    }
}

impl Expression {
    pub fn literal(text: String) -> Self {
        Self::Literal(text)
    }

    pub fn or(e: Vec<Expression>) -> Self {
        Self::Or(
            e.into_iter()
                .flat_map(|c| match c {
                    Self::Or(es) => es,
                    x => vec![x],
                })
                .collect(),
        )
    }

    pub fn and(e: Vec<Expression>) -> Self {
        Self::And(
            e.into_iter()
                .flat_map(|c| match c {
                    Self::And(es) => es,
                    x => vec![x],
                })
                .collect(),
        )
    }

    pub fn simplify(&self) -> Self {
        match self {
            Expression::Literal(_) => self.clone(),
            Expression::And(v) => {
                let v = v
                    .iter()
                    .flat_map(|e| {
                        let e = e.simplify();
                        if let Expression::And(inner) = e {
                            inner
                        } else {
                            vec![e]
                        }
                    })
                    .collect::<Vec<_>>();
                if v.len() == 1 {
                    v.into_iter().next().unwrap()
                } else {
                    Expression::And(v)
                }
            }
            Expression::Or(v) => {
                let v = v
                    .iter()
                    .flat_map(|e| {
                        let e = e.simplify();
                        if let Expression::Or(inner) = e {
                            inner
                        } else {
                            vec![e]
                        }
                    })
                    .collect::<Vec<_>>();
                if v.len() == 1 {
                    v.into_iter().next().unwrap()
                } else {
                    Expression::Or(v)
                }
            }
        }
    }

    /// convert the expression into disjunctive normal form
    ///
    /// careful, for some expressions this can have exponential runtime. E.g. the disjunctive normal form
    /// of `(a | b) & (c | d) & (e | f) & ...` will be very complex.
    pub fn dnf(self) -> Dnf {
        match self {
            Expression::Literal(x) => Dnf::literal(x),
            Expression::Or(es) => Reduce::reduce(es.into_iter().map(|x| x.dnf()), Dnf::bitor).unwrap(),
            Expression::And(es) => Reduce::reduce(es.into_iter().map(|x| x.dnf()), Dnf::bitand).unwrap(),
        }
    }
}

impl BitOr for Expression {
    type Output = Expression;
    fn bitor(self, that: Self) -> Self {
        Expression::or(vec![self, that])
    }
}

impl BitAnd for Expression {
    type Output = Expression;
    fn bitand(self, that: Self) -> Self {
        Expression::and(vec![self, that])
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Expression {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let size = g.size().min(3);
        let mut res = vec![];
        for _ in 0..size {
            if *g.choose(&[true, false, false, false, false]).unwrap() {
                res.push(Self::arbitrary(g));
            } else {
                let mut s = String::arbitrary(g);
                while s.is_empty() {
                    s = String::arbitrary(g);
                }
                res.push(Expression::Literal(s));
            }
        }
        if *g.choose(&[true, false]).unwrap() {
            Expression::And(res)
        } else {
            Expression::Or(res)
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        fn shrink_lvl(v: &[Expression]) -> impl Iterator<Item = Vec<Expression>> {
            // the idea is to first try removing single items from this vector, then
            // dive into the items and shrink those
            let first = {
                let v = v.to_owned();
                let end = if v.len() > 2 { v.len() } else { 0 };

                (0..end).map(move |i| {
                    let mut v = v.clone();
                    v.remove(i);
                    v
                })
            };
            let second = {
                let v = v.to_owned();

                (0..v.len()).flat_map(move |i| {
                    let v3 = v.clone();
                    v[i].shrink().map(move |e| {
                        let mut v = v3.clone();
                        v[i] = e;
                        v
                    })
                })
            };
            first.chain(second)
        }
        match self {
            Expression::Literal(s) => Box::new({
                let mut res = vec![];
                if s.as_str() != "a" {
                    res.push(Expression::literal("a".to_owned()));
                }
                res.into_iter()
            }),
            Expression::And(v) => Box::new(shrink_lvl(v).map(Expression::And)),
            Expression::Or(v) => Box::new(shrink_lvl(v).map(Expression::Or)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;
    use Expression::*;

    fn l(x: &str) -> Expression {
        Expression::literal(x.into())
    }

    #[test]
    fn single_literal() {
        let res: Expression = "'hello'".parse().unwrap();
        assert_eq!(res, Expression::Literal("hello".to_owned()));
    }

    #[test]
    fn no_empty_literals() {
        let err = "''".parse::<Expression>().unwrap_err();
        assert!(err.to_string().contains("NonEmpty"));
    }

    #[test]
    fn incomplete_literal() {
        let err = "'a".parse::<Expression>().unwrap_err();
        assert!(err.to_string().contains("expected ''', got end of input"));
    }

    #[test]
    fn toplevel_or() {
        let res = "'a'|('b'|'c')".parse::<Expression>().unwrap();
        assert_eq!(res, Or(vec![l("a"), l("b"), l("c")]));
    }

    #[test]
    fn toplevel_and() {
        let res = "'a'&('b'&'c')".parse::<Expression>().unwrap();
        assert_eq!(res, And(vec![l("a"), l("b"), l("c")]));
    }

    #[test]
    fn complex_expression() {
        let res = " ( 'a''' | 'b' ) & 'c' | 'd' ".parse::<Expression>().unwrap();
        assert_eq!(res, Or(vec![And(vec![Or(vec![l("a'"), l("b")]), l("c")]), l("d")]));
    }

    #[test]
    fn trailing_garbage() {
        let res = "'a' b".parse::<Expression>().unwrap_err();
        assert_eq!(res.to_string(), "trailing garbage: 'b'".to_owned());
    }

    quickcheck! {
        fn string_roundtrip(expr: Expression) -> bool {
            // print with minimum number of parens
            let s = expr.to_string();
            // parse back, now without redundant grouping
            let e = (&*s).parse::<Expression>().unwrap();
            // compare to suitably simplified expression
            expr.simplify() == e
        }
    }
}
