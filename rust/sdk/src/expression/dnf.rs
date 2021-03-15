/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::{
    collections::BTreeSet,
    ops::{BitAnd, BitOr},
};

use maplit::btreeset;
use reduce::Reduce;

use super::Expression;

/// Disjunctive normal form of a boolean query expression
///
/// https://en.wikipedia.org/wiki/Disjunctive_normal_form
///
/// This is an unique represenation of a query using literals, union and intersection.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Dnf(pub BTreeSet<BTreeSet<String>>);

impl Dnf {
    pub fn literal(text: String) -> Self {
        Self(btreeset![btreeset![text]])
    }

    /// converts the disjunctive normal form back to an expression
    pub fn expression(self) -> Expression {
        Reduce::reduce(self.0.into_iter().map(Dnf::and_expr), Expression::bitor).unwrap()
    }

    fn and_expr(v: BTreeSet<String>) -> Expression {
        Reduce::reduce(v.into_iter().map(Expression::literal), Expression::bitand).unwrap()
    }
}

fn insert_unless_redundant(aa: &mut BTreeSet<BTreeSet<String>>, b: BTreeSet<String>) {
    let mut to_remove = None;
    for a in aa.iter() {
        if a.is_subset(&b) {
            // a is larger than b. E.g. x | x&y
            // keep a, b is redundant
            return;
        } else if a.is_superset(&b) {
            // a is smaller than b, E.g. x&y | x
            // remove a, keep b
            to_remove = Some(a.clone());
        }
    }
    if let Some(r) = to_remove {
        aa.remove(&r);
    }
    aa.insert(b);
}

impl From<Expression> for Dnf {
    fn from(value: Expression) -> Self {
        value.dnf()
    }
}

impl BitAnd for Dnf {
    type Output = Dnf;
    fn bitand(self, that: Self) -> Self {
        let mut rs = BTreeSet::new();
        for a in self.0.iter() {
            for b in that.0.iter() {
                let mut r = BTreeSet::new();
                r.extend(a.iter().cloned());
                r.extend(b.iter().cloned());
                insert_unless_redundant(&mut rs, r);
            }
        }
        Dnf(rs)
    }
}

impl BitOr for Dnf {
    type Output = Dnf;
    fn bitor(self, that: Self) -> Self {
        let mut rs = self.0;
        for b in that.0 {
            insert_unless_redundant(&mut rs, b);
        }
        Dnf(rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(x: &str) -> Expression {
        Expression::literal(x.into())
    }

    #[test]
    fn test_dnf_intersection_1() {
        let a = l("a");
        let b = l("b");
        let c = l("c");
        let expr = c & (a | b);
        let c = expr.dnf().expression().to_string();
        assert_eq!(c, "'a' & 'c' | 'b' & 'c'");
    }

    #[test]
    fn test_dnf_intersection_2() {
        let a = l("a");
        let b = l("b");
        let c = l("c");
        let d = l("d");
        let expr = (d | c) & (b | a);
        let c = expr.dnf().expression().to_string();
        assert_eq!(c, "'a' & 'c' | 'a' & 'd' | 'b' & 'c' | 'b' & 'd'");
    }

    #[test]
    fn test_dnf_simplify_1() {
        let a = l("a");
        let b = l("b");
        let expr = (a.clone() | b) & a;
        let c = expr.dnf().expression().to_string();
        assert_eq!(c, "'a'");
    }

    #[test]
    fn test_dnf_simplify_2() {
        let a = l("a");
        let b = l("b");
        let expr = (a.clone() & b) | a;
        let c = expr.dnf().expression().to_string();
        assert_eq!(c, "'a'");
    }

    #[test]
    fn test_dnf_simplify_3() {
        let a = l("a");
        let b = l("b");
        let expr = (a.clone() | b) | a;
        let c = expr.dnf().expression().to_string();
        assert_eq!(c, "'a' | 'b'");
    }
}
