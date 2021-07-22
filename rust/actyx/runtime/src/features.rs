use crate::{operation::Operation, query::Query};
use actyx_sdk::language::{Arr, Ind, Obj, SimpleExpr, TagAtom, TagExpr};
use std::{collections::BTreeSet, str::FromStr};

#[derive(Debug, Clone, derive_more::Display, PartialEq)]
pub enum FeatureError {
    #[display(fmt = "The query uses unreleased features: {}.", _0)]
    Alpha(String),
    #[display(fmt = "The query uses beta features that are not enabled: {}.", _0)]
    Beta(String),
    #[display(fmt = "Feature(s) `{}` are not supported on endpoint {}.", features, endpoint)]
    #[allow(dead_code)]
    Unsupported { features: String, endpoint: String },
}
impl std::error::Error for FeatureError {}

#[derive(Debug, Clone, PartialEq)]
pub enum FeatureKind {
    Released,
    Beta,
    Alpha,
}

macro_rules! features {
    ($($name:ident: $kind:ident,)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[allow(non_camel_case_types)]
        pub enum Feature {
            $($name,)*
        }
        impl Feature {
            pub fn kind(self) -> FeatureKind {
                match self {
                    $(Feature::$name => FeatureKind::$kind,)*
                }
            }
        }
        impl FromStr for Feature {
            type Err = ();
            fn from_str(s: &str) -> Result<Feature, ()> {
                match s {
                    $(stringify!($name) => Ok(Feature::$name),)*
                    _ => Err(()),
                }
            }
        }
        impl std::fmt::Display for Feature {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($name => write!(f, stringify!($name)),)*
                }
            }
        }
    };
}

features! {
    timeRange: Beta,
    eventKeyRange: Beta,
    multiEmission: Alpha,
}

use itertools::Itertools;
use Feature::*;

#[derive(Debug, Clone, Default)]
pub struct Features(BTreeSet<Feature>);

impl Features {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_query(q: &Query) -> Self {
        let mut features = Self::new();
        features_tag(&mut features, &q.from);
        for op in q.stages.iter() {
            features_op(&mut features, op);
        }
        features
    }

    pub fn add(&mut self, f: Feature) {
        self.0.insert(f);
    }

    pub fn validate(&self, enabled: &[String]) -> Result<(), FeatureError> {
        let mut alpha = false;
        let mut enabled_features = BTreeSet::new();
        for s in enabled {
            if s == "zøg" {
                alpha = true;
            } else if let Ok(f) = Feature::from_str(s) {
                enabled_features.insert(f);
            } // ignore unknown features — might be from the future
        }
        // first check whether alpha features were enabled without enabling alpha itself
        if !alpha && enabled_features.iter().any(|f| f.kind() == FeatureKind::Alpha) {
            return Err(FeatureError::Alpha(
                enabled_features
                    .iter()
                    .filter(|f| f.kind() == FeatureKind::Alpha)
                    .join(" "),
            ));
        }
        // then check whether alpha/beta features were used but not enabled
        let mut a = vec![];
        let mut b = vec![];
        for f in self.0.difference(&enabled_features) {
            match f.kind() {
                FeatureKind::Released => {}
                FeatureKind::Beta => b.push(*f),
                FeatureKind::Alpha => a.push(*f),
            }
        }
        if !a.is_empty() {
            return Err(FeatureError::Alpha(a.iter().join(" ")));
        }
        if !b.is_empty() {
            return Err(FeatureError::Beta(b.iter().join(" ")));
        }
        Ok(())
    }
}

fn features_op(feat: &mut Features, op: &Operation) {
    match op {
        Operation::Filter(f) => features_simple(feat, &f.expr),
        Operation::Select(s) => {
            if s.exprs.len() > 1 {
                feat.add(multiEmission);
            }
            for expr in &s.exprs {
                features_simple(feat, expr);
            }
        }
    }
}

fn features_tag(feat: &mut Features, expr: &TagExpr) {
    match expr {
        TagExpr::Or(x) => {
            features_tag(feat, &x.0);
            features_tag(feat, &x.1);
        }
        TagExpr::And(x) => {
            features_tag(feat, &x.0);
            features_tag(feat, &x.1);
        }
        TagExpr::Atom(a) => match a {
            TagAtom::Tag(_) => {}
            TagAtom::AllEvents => {}
            TagAtom::IsLocal => {}
            TagAtom::FromTime(_) => feat.add(timeRange),
            TagAtom::ToTime(_) => feat.add(timeRange),
            TagAtom::FromLamport(_) => feat.add(eventKeyRange),
            TagAtom::ToLamport(_) => feat.add(eventKeyRange),
            TagAtom::AppId(_) => {}
        },
    }
}

fn features_simple(feat: &mut Features, expr: &SimpleExpr) {
    match expr {
        SimpleExpr::Variable(_string) => {}
        SimpleExpr::Indexing(Ind { head, tail }) => {
            features_simple(feat, head);
            for idx in tail.iter() {
                match idx {
                    actyx_sdk::language::Index::String(_) => {}
                    actyx_sdk::language::Index::Number(_) => {}
                    actyx_sdk::language::Index::Expr(expr) => {
                        features_simple(feat, expr);
                    }
                }
            }
        }
        SimpleExpr::Number(_n) => {}
        SimpleExpr::String(_string) => {}
        SimpleExpr::Object(Obj { props }) => {
            for (idx, expr) in props {
                features_simple(feat, expr);
                match idx {
                    actyx_sdk::language::Index::String(_) => {}
                    actyx_sdk::language::Index::Number(_) => {}
                    actyx_sdk::language::Index::Expr(expr) => {
                        features_simple(feat, expr);
                    }
                }
            }
        }
        SimpleExpr::Array(Arr { items }) => {
            for expr in items {
                features_simple(feat, expr);
            }
        }
        SimpleExpr::Null => {}
        SimpleExpr::Bool(_) => {}
        SimpleExpr::Cases(v) => {
            for (pred, res) in v.iter() {
                features_simple(feat, pred);
                features_simple(feat, res);
            }
        }
        SimpleExpr::Add(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Sub(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Mul(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Div(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Mod(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Pow(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::And(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Or(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Xor(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Lt(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Le(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Gt(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Ge(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Eq(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Ne(x) => {
            features_simple(feat, &x.0);
            features_simple(feat, &x.1);
        }
        SimpleExpr::Not(x) => features_simple(feat, x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use FeatureError::*;

    fn s(s: &str) -> String {
        String::from(s)
    }
    fn q(s: &str) -> Result<(), FeatureError> {
        let q = Query::from(s.parse::<actyx_sdk::language::Query>().unwrap());
        Features::from_query(&q).validate(&q.features)
    }

    #[test]
    fn alpha() {
        let f = Features::new();

        assert_eq!(f.validate(&[s("multiEmission")]), Err(Alpha(s("multiEmission"))));
        assert_eq!(f.validate(&[s("zøg"), s("multiEmission")]), Ok(()));
        assert_eq!(f.validate(&[s("multiEmission"), s("zøg")]), Ok(()));
    }

    #[test]
    fn time_range() {
        assert_eq!(q("FROM from(2021-07-07Z)"), Err(Beta(s("timeRange"))));
        assert_eq!(q("FROM to(2021-07-07Z)"), Err(Beta(s("timeRange"))));

        assert_eq!(q("FEATURES(timeRange) FROM from(2021-07-07Z)"), Ok(()));
        assert_eq!(q("FEATURES(timeRange) FROM to(2021-07-07Z)"), Ok(()));
    }

    #[test]
    fn event_key_range() {
        assert_eq!(q("FROM from(12345)"), Err(Beta(s("eventKeyRange"))));
        assert_eq!(q("FROM to(12345)"), Err(Beta(s("eventKeyRange"))));

        assert_eq!(q("FEATURES(eventKeyRange) FROM from(12345)"), Ok(()));
        assert_eq!(q("FEATURES(eventKeyRange) FROM to(12345)"), Ok(()));
    }

    #[test]
    fn multi_emission() {
        assert_eq!(q("FROM allEvents SELECT _, _"), Err(Alpha(s("multiEmission"))));
        assert_eq!(
            q("FEATURES(multiEmission) FROM allEvents SELECT _, _"),
            Err(Alpha(s("multiEmission")))
        );
        assert_eq!(q("FEATURES(multiEmission zøg) FROM allEvents SELECT _, _"), Ok(()));
    }
}
