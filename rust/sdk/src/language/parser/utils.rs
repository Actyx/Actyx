use super::{NoVal, Rule};
use crate::language::non_empty::NonEmptyString;
use anyhow::Result;
use pest::{
    error::Error,
    iterators::{Pair, Pairs},
};
use std::{convert::TryInto, fmt::Debug, str::FromStr};

pub type R<T> = std::result::Result<T, Error<Rule>>;
pub type Ps<'a> = Pairs<'a, Rule>;
pub type P<'a> = Pair<'a, Rule>;

pub trait Ext<'a>: 'a {
    fn single(self) -> Result<P<'a>>;
    fn inner(self) -> Result<Ps<'a>>;
    fn rule(&self) -> Result<Rule>;
    fn string(&mut self) -> Result<String>;
    fn non_empty_string(&mut self) -> Result<NonEmptyString>;
    fn natural(&mut self) -> Result<u64>;
    fn decimal(&mut self) -> Result<f64>;
    /// if exists, parse as string and return error
    fn parse_or_default<T>(&mut self) -> Result<T>
    where
        T: Default + FromStr + Debug,
        anyhow::Error: From<T::Err>,
        T::Err: Send + Sync + 'static;
}

impl<'a> Ext<'a> for Ps<'a> {
    fn single(mut self) -> Result<P<'a>> {
        Ok(self.next().ok_or(NoVal(""))?)
    }

    fn inner(self) -> Result<Ps<'a>> {
        Ok(self.single()?.into_inner())
    }

    fn rule(&self) -> Result<Rule> {
        Ok(self.peek().ok_or(NoVal("peek"))?.as_rule())
    }

    fn string(&mut self) -> Result<String> {
        Ok(self.next().ok_or(NoVal("string"))?.as_str().to_owned())
    }

    fn non_empty_string(&mut self) -> Result<NonEmptyString> {
        Ok(self
            .next()
            .ok_or(NoVal("non_empty_string"))?
            .as_str()
            .to_owned()
            .try_into()?)
    }

    fn natural(&mut self) -> Result<u64> {
        Ok(self.next().ok_or(NoVal("natural"))?.as_str().parse()?)
    }

    fn decimal(&mut self) -> Result<f64> {
        Ok(self.next().ok_or(NoVal("decimal"))?.as_str().parse()?)
    }

    fn parse_or_default<T>(&mut self) -> Result<T>
    where
        T: Default + FromStr + Debug,
        anyhow::Error: From<T::Err>,
        T::Err: Send + Sync + 'static,
    {
        Ok(self
            .next()
            .map(|o| o.as_str().parse::<T>())
            .transpose()?
            .unwrap_or_default())
    }
}

impl<'a> Ext<'a> for P<'a> {
    fn single(self) -> Result<P<'a>> {
        Ok(self.inner()?.next().ok_or(NoVal("single"))?)
    }

    fn inner(self) -> Result<Ps<'a>> {
        Ok(self.into_inner())
    }

    fn rule(&self) -> Result<Rule> {
        Ok(self.as_rule())
    }

    fn string(&mut self) -> Result<String> {
        Ok(self.as_str().to_owned())
    }

    fn non_empty_string(&mut self) -> Result<NonEmptyString> {
        Ok(self.as_str().try_into()?)
    }

    fn natural(&mut self) -> Result<u64> {
        Ok(self.as_str().parse()?)
    }

    fn decimal(&mut self) -> Result<f64> {
        Ok(self.as_str().parse()?)
    }

    fn parse_or_default<T>(&mut self) -> Result<T>
    where
        T: Default + FromStr + Debug,
        anyhow::Error: From<T::Err>,
        T::Err: Send + Sync + 'static,
    {
        Ok(self.as_str().parse::<T>()?)
    }
}
