use super::Rule;
use crate::language::non_empty::NonEmptyString;
use pest::{
    error::Error,
    iterators::{Pair, Pairs},
};
use std::{convert::TryInto, fmt::Debug, str::FromStr};

pub type R<T> = std::result::Result<T, Error<Rule>>;
pub type Ps<'a> = Pairs<'a, Rule>;
pub type P<'a> = Pair<'a, Rule>;

pub trait Ext<'a>: 'a {
    fn single(self) -> P<'a>;
    fn inner(self) -> Ps<'a>;
    fn rule(&self) -> Rule;
    fn string(&mut self) -> String;
    fn non_empty_string(&mut self) -> NonEmptyString;
    fn natural(&mut self) -> Option<u64>;
    fn decimal(&mut self) -> f64;
    fn parse_or_default<T>(&mut self) -> T
    where
        T: Default + FromStr + Debug,
        T::Err: Debug;
}

impl<'a> Ext<'a> for Ps<'a> {
    fn single(mut self) -> P<'a> {
        self.next().unwrap()
    }

    fn inner(self) -> Ps<'a> {
        self.single().into_inner()
    }

    fn rule(&self) -> Rule {
        self.peek().unwrap().as_rule()
    }

    fn string(&mut self) -> String {
        self.next().unwrap().as_str().to_owned()
    }

    fn non_empty_string(&mut self) -> NonEmptyString {
        self.next().unwrap().as_str().to_owned().try_into().unwrap()
    }

    fn natural(&mut self) -> Option<u64> {
        self.next().unwrap().as_str().parse().ok()
    }

    fn decimal(&mut self) -> f64 {
        self.next().unwrap().as_str().parse().unwrap()
    }

    fn parse_or_default<T>(&mut self) -> T
    where
        T: Default + FromStr + Debug,
        T::Err: Debug,
    {
        self.next()
            .map(|o| o.as_str().parse::<T>().unwrap())
            .unwrap_or_default()
    }
}

impl<'a> Ext<'a> for P<'a> {
    fn single(self) -> P<'a> {
        self.inner().next().unwrap()
    }

    fn inner(self) -> Ps<'a> {
        self.into_inner()
    }

    fn rule(&self) -> Rule {
        self.as_rule()
    }

    fn string(&mut self) -> String {
        self.as_str().to_owned()
    }

    fn non_empty_string(&mut self) -> NonEmptyString {
        self.as_str().to_owned().try_into().unwrap()
    }

    fn natural(&mut self) -> Option<u64> {
        self.as_str().parse().ok()
    }

    fn decimal(&mut self) -> f64 {
        self.as_str().parse().unwrap()
    }

    fn parse_or_default<T>(&mut self) -> T
    where
        T: Default + FromStr + Debug,
        T::Err: Debug,
    {
        self.as_str().parse::<T>().unwrap()
    }
}
