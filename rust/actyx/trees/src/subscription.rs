use actyxos_sdk::{tags, Expression, Tag, TagSet};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
/// One particular intersection of tags selected for subscription.
/// Setting the local flag selects only sources from the local node.
#[serde(rename_all = "camelCase")]
pub struct TagSubscription {
    pub tags: TagSet,
    pub local: bool,
}
impl TagSubscription {
    pub fn new(tags: TagSet) -> Self {
        Self { tags, local: false }
    }
    pub fn local(mut self) -> Self {
        self.local = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagSubscriptions(Vec<TagSubscription>);
impl TagSubscriptions {
    pub fn all() -> Self {
        // Empty set is the subset of all sets
        Self(vec![TagSubscription::new(tags!())])
    }
    pub fn empty() -> Self {
        Self(vec![])
    }
    pub fn new(s: Vec<TagSubscription>) -> Self {
        Self(s)
    }
    pub fn only_local(&self) -> bool {
        !self.0.is_empty() && self.0.iter().all(|x| x.local)
    }
}
impl From<Expression> for TagSubscriptions {
    fn from(e: Expression) -> Self {
        let dnf = e.dnf();
        Self(
            dnf.0
                .into_iter()
                .map(|tag_set| TagSubscription {
                    tags: tag_set
                        .into_iter()
                        .map(|x| Tag::new(x).expect("dnf does not emit empty tags"))
                        .collect(),
                    local: false,
                })
                .collect(),
        )
    }
}
impl Deref for TagSubscriptions {
    type Target = Vec<TagSubscription>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for TagSubscriptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<TagSubscriptions> for Vec<TagSet> {
    fn from(ts: TagSubscriptions) -> Vec<TagSet> {
        ts.0.into_iter().map(|x| x.tags).collect()
    }
}
