use super::Rule;
use pest::{
    error::Error,
    iterators::{Pair, Pairs},
};

pub type R<T> = std::result::Result<T, Error<Rule>>;
pub type Ps<'a> = Pairs<'a, Rule>;
pub type P<'a> = Pair<'a, Rule>;

pub trait Ext<'a>: 'a {
    fn single(self) -> P<'a>;
    fn inner(self) -> Ps<'a>;
    fn rule(&self) -> Rule;
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
}
