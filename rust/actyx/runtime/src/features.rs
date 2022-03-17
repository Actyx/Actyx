use crate::{operation::Operation, query::Query};
use actyx_sdk::language::{SimpleExpr, TagAtom, TagExpr, Traverse};
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
    ($($name:ident: $kind:ident [$($endpoint:ident)*],)+) => {
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

            pub fn valid_on_endpoint(self, ep: Endpoint) -> Result<(), FeatureError> {
                match self {
                    $(Feature::$name => match ep {
                        $(Endpoint::$endpoint => Err(FeatureError::Unsupported { features: stringify!($name).to_owned(), endpoint: ep.to_string() }),)*
                        _ => Ok(()),
                    },)*
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
    timeRange: Beta [],
    eventKeyRange: Beta [],
    multiEmission: Alpha [Subscribe SubscribeMonotonic],
    aggregate: Alpha [Subscribe SubscribeMonotonic],
    subQuery: Alpha [Subscribe SubscribeMonotonic],
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum Endpoint {
    Query,
    Subscribe,
    SubscribeMonotonic,
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

    pub fn validate(&self, enabled: &[String], endpoint: Endpoint) -> Result<(), FeatureError> {
        let mut alpha = false;
        let mut enabled_features = BTreeSet::new();
        for s in enabled {
            if s == "zøg" || s == "zoeg" {
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
        // last check if all used features are valid on the endpoint
        for f in self.0.iter() {
            f.valid_on_endpoint(endpoint)?;
        }
        Ok(())
    }
}

fn features_op(feat: &mut Features, op: &Operation) {
    match op {
        Operation::Filter(f) => features_simple(feat, f),
        Operation::Select(s) => {
            if s.len() > 1 {
                feat.add(multiEmission);
            }
            for expr in s.iter() {
                features_simple(feat, expr);
            }
        }
        Operation::Aggregate(a) => {
            feat.add(aggregate);
            features_simple(feat, a);
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
    expr.traverse(&mut |e| match e {
        SimpleExpr::SubQuery(q) => {
            features_tag(feat, &q.from);
            for op in q.ops.iter() {
                features_op(feat, &Operation::from(op.clone()));
            }
            feat.add(subQuery);
            Traverse::Descend
        }
        _ => Traverse::Descend,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::btreeset;
    use FeatureError::*;

    fn s(s: &str) -> String {
        String::from(s)
    }
    fn f(s: &str) -> Features {
        let q = Query::from(s.parse::<actyx_sdk::language::Query>().unwrap());
        Features::from_query(&q)
    }
    fn q(s: &str) -> Result<(), FeatureError> {
        let q = Query::from(s.parse::<actyx_sdk::language::Query>().unwrap());
        Features::from_query(&q).validate(&q.features, Endpoint::Query)
    }

    #[test]
    fn alpha() {
        let f = Features::new();

        assert_eq!(
            f.validate(&[s("multiEmission")], Endpoint::Query),
            Err(Alpha(s("multiEmission")))
        );
        assert_eq!(f.validate(&[s("zøg"), s("multiEmission")], Endpoint::Query), Ok(()));
        assert_eq!(f.validate(&[s("multiEmission"), s("zøg")], Endpoint::Query), Ok(()));
        assert_eq!(f.validate(&[s("zoeg"), s("multiEmission")], Endpoint::Query), Ok(()));
        assert_eq!(f.validate(&[s("multiEmission"), s("zoeg")], Endpoint::Query), Ok(()));
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

        let mut f = Features::new();
        f.add(Feature::multiEmission);
        assert_eq!(
            f.validate(&[s("zøg"), s("multiEmission")], Endpoint::Subscribe),
            Err(Unsupported {
                features: s("multiEmission"),
                endpoint: s("Subscribe")
            })
        );
    }

    #[test]
    fn aggr() {
        let mut f = Features::new();
        f.add(Feature::aggregate);
        assert_eq!(
            f.validate(&[s("zøg"), s("aggregate")], Endpoint::Subscribe),
            Err(Unsupported {
                features: s("aggregate"),
                endpoint: s("Subscribe")
            })
        );
    }

    #[test]
    fn subquery() {
        assert_eq!(f("FROM 'x' SELECT 1 + (FROM 'y' END)[0]").0, btreeset!(subQuery));
        assert_eq!(f("FROM 'x' FILTER 1 + (FROM 'y' END)[0]").0, btreeset!(subQuery));
        assert_eq!(
            f("FROM 'x' AGGREGATE 1 + (FROM 'y' END)[0]").0,
            btreeset!(aggregate, subQuery)
        );
        assert_eq!(
            f("FROM 'x' SELECT 1 + (FROM 'y' AGGREGATE 42 END)[0]").0,
            btreeset!(aggregate, subQuery)
        );
        assert_eq!(
            f("FROM 'x' SELECT 1 + (FROM 'y' SELECT 1, 2 END)[0]").0,
            btreeset!(multiEmission, subQuery)
        );
        assert_eq!(
            f("FROM 'x' SELECT 1 + (FROM 'y' SELECT 1, FROM 'a' AGGREGATE x)[0]").0,
            btreeset!(multiEmission, aggregate, subQuery)
        );
    }
}
