use super::parser::{NoVal, Rule};
use crate::language::non_empty::NonEmptyString;
use anyhow::Result;
use pest::{
    error::ErrorVariant,
    iterators::{Pair, Pairs},
};
use std::{
    convert::TryInto,
    fmt::Debug,
    ops::{Deref, DerefMut},
    str::FromStr,
};

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

pub trait Spanned {
    type Ret;
    fn spanned(self, span: pest::Span) -> Self::Ret;
}

impl<T, E: ToString> Spanned for Result<T, E> {
    type Ret = Result<T, pest::error::Error<Rule>>;
    fn spanned(self, span: pest::Span) -> Self::Ret {
        self.map_err(|e| {
            let e = ErrorVariant::CustomError { message: e.to_string() };
            pest::error::Error::new_from_span(e, span)
        })
    }
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct Span<'a, T> {
    span: pest::Span<'a>,
    value: T,
}

impl<'a, T: PartialEq> PartialEq for Span<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'a, T: Ord> Ord for Span<'a, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<'a, T: Ord> PartialOrd for Span<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, T> Span<'a, T> {
    pub fn new(span: pest::Span<'a>, value: T) -> Self {
        Self { span, value }
    }

    pub fn make(p: P<'a>, f: impl FnOnce(P<'a>) -> Result<T>) -> Result<Self> {
        Ok(Self::new(p.as_span(), f(p)?))
    }

    #[allow(clippy::result_large_err)]
    pub fn err(&self, msg: impl Into<String>) -> Result<(), pest::error::Error<Rule>> {
        Err(pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError { message: msg.into() },
            self.span,
        ))
    }

    pub fn span(&self) -> pest::Span<'a> {
        self.span
    }
}

impl<'a, T> Deref for Span<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> DerefMut for Span<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
