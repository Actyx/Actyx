use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display, iter::FromIterator, str::FromStr};

#[derive(Debug, Clone, Default)]
pub struct RadixTree<V> {
    // todo: make this a proper radix tree!
    substitutions: BTreeMap<String, V>,
}

impl<V: Serialize> Serialize for RadixTree<V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.substitutions.serialize(serializer)
    }
}

impl<'de, V: DeserializeOwned> Deserialize<'de> for RadixTree<V> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let substitutions = <BTreeMap<String, V>>::deserialize(deserializer)?;
        Ok(Self { substitutions })
    }
}

impl<V: DeserializeOwned> FromStr for RadixTree<V> {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let substitutions: BTreeMap<String, V> = serde_json::from_str(&text).context("unexpected json structure")?;
        Ok(Self { substitutions })
    }
}

impl<V: Serialize> Display for RadixTree<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = if f.alternate() {
            serde_json::to_string_pretty(&self).unwrap()
        } else {
            serde_json::to_string(&self).unwrap()
        };
        write!(f, "{}", text)
    }
}

impl<V: Default> RadixTree<V> {
    /// Get the value at exactly this key, or insert the default and return a reference to it
    pub fn get_or_insert_default(&mut self, key: String) -> &mut V {
        self.substitutions.entry(key).or_default()
    }
}

impl<V> RadixTree<V> {
    /// Add a new mapping
    pub fn insert(&mut self, prefix: String, value: V) {
        self.substitutions.insert(prefix, value);
    }

    /// Look up and return the best match
    pub fn get(&self, text: &str) -> Option<&V> {
        self.best_match(text).map(|(_, v)| v)
    }

    /// Find the best match (with the longest prefix) and just return it including the found prefix
    pub fn best_match(&self, text: &str) -> Option<(&str, &V)> {
        self.substitutions
            .iter()
            .filter(|(prefix, _)| text.starts_with(*prefix))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(k, v)| (k.as_str(), v))
    }
}

impl RadixTree<String> {
    pub fn new() -> Self {
        Self {
            substitutions: Default::default(),
        }
    }

    pub fn prefix(self, key_prefix: &str, value_prefix: &str) -> Self {
        Self {
            substitutions: self
                .substitutions
                .into_iter()
                .map(|(k, v)| (format!("{}{}", key_prefix, k), format!("{}{}", value_prefix, v)))
                .collect(),
        }
    }

    /// compute the inverse
    pub fn inverse(&self) -> RadixTree<String> {
        RadixTree {
            substitutions: self
                .substitutions
                .iter()
                .map(|(prefix, substitution)| (substitution.clone(), prefix.clone()))
                .collect(),
        }
    }

    /// Look up a string, and if found, perform the substitution
    pub fn substitute(&self, text: &str) -> Option<String> {
        if let Some((prefix, substitution)) = self.best_match(text) {
            Some(format!("{}{}", substitution, &text[prefix.len()..]))
        } else {
            None
        }
    }
}

impl<V> FromIterator<(String, V)> for RadixTree<V> {
    fn from_iter<T: IntoIterator<Item = (String, V)>>(iter: T) -> Self {
        Self {
            substitutions: iter.into_iter().collect(),
        }
    }
}
