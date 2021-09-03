use actyx_sdk::{
    language::{TagAtom, TagExpr},
    AppId, Tag, TagSet,
};
use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, convert::TryFrom, fmt::Display, iter::FromIterator, str::FromStr, sync::Arc};

/// Utility for doing most specific prefix lookups and substitutions
#[derive(Debug, Clone, Default)]
pub struct PrefixSubstitution<V> {
    substitutions: BTreeMap<String, V>,
}

impl<V: Serialize> Serialize for PrefixSubstitution<V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.substitutions.serialize(serializer)
    }
}

impl<'de, V: DeserializeOwned> Deserialize<'de> for PrefixSubstitution<V> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let substitutions = <BTreeMap<String, V>>::deserialize(deserializer)?;
        Ok(Self { substitutions })
    }
}

impl<V: DeserializeOwned> FromStr for PrefixSubstitution<V> {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let substitutions: BTreeMap<String, V> = serde_json::from_str(&text).context("unexpected json structure")?;
        Ok(Self { substitutions })
    }
}

impl<V: Serialize> Display for PrefixSubstitution<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = if f.alternate() {
            serde_json::to_string_pretty(&self).unwrap()
        } else {
            serde_json::to_string(&self).unwrap()
        };
        write!(f, "{}", text)
    }
}

impl<V: DeserializeOwned> PrefixSubstitution<V> {
    pub fn parse_json(json: serde_json::Value) -> anyhow::Result<Self> {
        let text = serde_json::to_string(&json)?;
        let substitutions: BTreeMap<String, V> = serde_json::from_str(&text).context("unexpected json structure")?;
        Ok(Self { substitutions })
    }
}

impl<V: Default> PrefixSubstitution<V> {
    /// Get the value at exactly this key, or insert the default and return a reference to it
    pub fn get_or_insert_default(&mut self, key: String) -> &mut V {
        self.substitutions.entry(key).or_default()
    }
}

impl<V> PrefixSubstitution<V> {
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

impl PrefixSubstitution<String> {
    pub fn new() -> Self {
        Self {
            substitutions: Default::default(),
        }
    }

    /// compute the inverse
    pub fn inverse(&self) -> PrefixSubstitution<String> {
        PrefixSubstitution {
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

impl<V> FromIterator<(String, V)> for PrefixSubstitution<V> {
    fn from_iter<T: IntoIterator<Item = (String, V)>>(iter: T) -> Self {
        Self {
            substitutions: iter.into_iter().collect(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TagMapper {
    to_global: BTreeMap<AppId, PrefixSubstitution<PrefixSubstitution<String>>>,
    to_local: BTreeMap<AppId, PrefixSubstitution<PrefixSubstitution<String>>>,
    write_to_global: BTreeMap<AppId, PrefixSubstitution<PrefixSubstitution<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingMode {
    /// you can only read this prefix
    R,
    /// you can read and write under this prefix
    RW,
    // not sure this makes any sense at all.
    // W,
}

impl<'de> serde::Deserialize<'de> for MappingMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let str = <String>::deserialize(deserializer)?;
        match str.as_str() {
            "r" => Ok(Self::R),
            "rw" => Ok(Self::RW),
            _ => Err(serde::de::Error::custom("Unexpected enum variant!")),
        }
    }
}

pub fn transform_atoms(expr: &TagExpr, f: &impl Fn(&TagAtom) -> anyhow::Result<TagAtom>) -> anyhow::Result<TagExpr> {
    Ok(match expr {
        TagExpr::Or(p) => TagExpr::Or(Arc::new((transform_atoms(&p.0, f)?, transform_atoms(&p.1, f)?))),
        TagExpr::And(p) => TagExpr::And(Arc::new((transform_atoms(&p.0, f)?, transform_atoms(&p.1, f)?))),
        TagExpr::Atom(atom) => TagExpr::Atom(f(atom)?),
    })
}

fn format_tag_set(tags: &TagSet) -> String {
    let tags = tags.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    format!("{{{}}}", tags.join(" "))
}

impl TagMapper {
    pub fn hardcoded() -> &'static TagMapper {
        &DEFAULT_MAPPER
    }

    pub fn get_mapping(&self, app_id: &AppId, app_version: &str) -> Option<&PrefixSubstitution<String>> {
        self.to_global
            .get(app_id)
            .and_then(|versions| versions.get(app_version))
    }

    pub fn tag_set_to_global(&self, app_id: &AppId, app_version: &str, tags: &TagSet) -> TagSet {
        let result = tags
            .iter()
            .filter_map(|tag| self.tag_to_global(app_id, app_version, &tag))
            .collect();
        tracing::info!(
            "app_id:'{}' app_version:'{}' {} => {}",
            app_id,
            app_version,
            format_tag_set(tags),
            format_tag_set(&result)
        );
        result
    }

    pub fn tag_set_write_to_global(&self, app_id: &AppId, app_version: &str, tags: &TagSet) -> anyhow::Result<TagSet> {
        let result = tags
            .iter()
            .map(|tag| self.tag_write_to_global(app_id, app_version, &tag))
            .collect::<anyhow::Result<_>>()?;
        tracing::info!(
            "app_id:'{}' app_version:'{}' {} => {}",
            app_id,
            app_version,
            format_tag_set(tags),
            format_tag_set(&result)
        );
        Ok(result)
    }

    pub fn tag_set_to_local(&self, app_id: &AppId, app_version: &str, tags: &TagSet) -> TagSet {
        tags.iter()
            .filter_map(|tag| self.tag_to_local(app_id, app_version, &tag))
            .collect()
    }

    pub fn tag_expr_to_global(&self, app_id: &AppId, app_version: &str, expr: &TagExpr) -> anyhow::Result<TagExpr> {
        let result = transform_atoms(expr, &|atom| {
            Ok(match atom {
                TagAtom::Tag(tag) => TagAtom::Tag(self.tag_to_global(app_id, app_version, tag).context("")?),
                other => other.clone(),
            })
        });
        if let Ok(translated) = &result {
            tracing::info!(
                "app_id:'{}' app_version:'{}' local:\"{}\" global:\"{}\"",
                app_id,
                app_version,
                expr,
                translated
            );
        }
        result
    }

    pub fn tag_to_global(&self, app_id: &AppId, app_version: &str, tag: &Tag) -> Option<Tag> {
        self.to_global(app_id, app_version, tag.as_ref())
            .and_then(|text| Tag::try_from(text.as_ref()).ok())
    }

    pub fn tag_write_to_global(&self, app_id: &AppId, app_version: &str, tag: &Tag) -> anyhow::Result<Tag> {
        let text = self
            .write_to_global(app_id, app_version, tag.as_ref())
            .context("No mapping configured")?;
        let tag = Tag::try_from(text.as_ref()).context("mapped to empty string")?;
        Ok(tag)
    }

    pub fn tag_to_local(&self, app_id: &AppId, app_version: &str, tag: &Tag) -> Option<Tag> {
        self.to_local(app_id, app_version, tag.as_ref())
            .and_then(|text| Tag::try_from(text.as_ref()).ok())
    }

    fn to_global(&self, app_id: &AppId, app_version: &str, text: &str) -> Option<String> {
        self.to_global
            .get(app_id)
            .and_then(|versions| versions.get(app_version))
            .and_then(|substitutions| substitutions.substitute(text))
    }

    fn write_to_global(&self, app_id: &AppId, app_version: &str, text: &str) -> Option<String> {
        self.write_to_global
            .get(app_id)
            .and_then(|versions| versions.get(app_version))
            .and_then(|substitutions| substitutions.substitute(text))
    }

    fn to_local(&self, app_id: &AppId, app_version: &str, text: &str) -> Option<String> {
        self.to_local
            .get(app_id)
            .and_then(|versions| versions.get(app_version))
            .and_then(|substitutions| substitutions.substitute(text))
    }

    fn add_mapping(
        &mut self,
        app_id: AppId,
        app_version: String,
        read_mapping: PrefixSubstitution<String>,
        write_mapping: PrefixSubstitution<String>,
    ) {
        self.to_local
            .entry(app_id.clone())
            .or_default()
            .insert(app_version.clone(), read_mapping.inverse());
        self.to_global
            .entry(app_id.clone())
            .or_default()
            .insert(app_version.clone(), read_mapping);
        self.write_to_global
            .entry(app_id)
            .or_default()
            .insert(app_version, write_mapping);
    }

    pub fn parse_json(value: serde_json::Value) -> anyhow::Result<Self> {
        let text = serde_json::to_string(&value)?;
        let data: BTreeMap<AppId, BTreeMap<String, BTreeMap<String, String>>> =
            serde_json::from_str(&text).context("unexpected json structure")?;
        let mut result = Self::default();
        for (app_id, versions) in data {
            for (version, mappings) in versions {
                result.add_mapping(
                    app_id.clone(),
                    version,
                    mappings.iter().map(|(x, y)| (x.clone(), y.clone())).collect(),
                    mappings.into_iter().collect(),
                );
            }
        }
        Ok(result)
    }

    pub fn parse_extended_json(value: serde_json::Value) -> anyhow::Result<Self> {
        let text = serde_json::to_string(&value)?;
        let data: BTreeMap<AppId, BTreeMap<String, BTreeMap<String, (String, MappingMode)>>> =
            serde_json::from_str(&text).context("unexpected json structure")?;
        let mut result = Self::default();
        for (app_id, versions) in data {
            for (app_version, mappings) in versions {
                let write_mapping = mappings
                    .iter()
                    .filter_map(|(prefix, (mapping, mode))| {
                        if *mode == MappingMode::RW {
                            Some((prefix.clone(), mapping.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                // everything goes into the read mapping for now.
                // not sure what write only would be good for.
                let read_mapping = mappings
                    .into_iter()
                    .map(|(prefix, (mapping, _))| (prefix, mapping))
                    .collect();
                result.add_mapping(app_id.clone(), app_version, read_mapping, write_mapping);
            }
        }
        Ok(result)
    }
}

lazy_static::lazy_static! {
    pub static ref DEFAULT_MAPPER: TagMapper = TagMapper::parse_extended_json(
        json!{
            {
                // converter from v1 to v2
                "com.example.todo-react-actyx-v1-v2": {
                    "": {
                        // from is mounted readonly
                        "from/": ["com.example.todo-react-actyx-v1/", "r"],
                        // to is mounted rw
                        "to/": ["com.example.todo-react-actyx-v2/", "rw"],
                        // the converter has its own mount to store progress info
                        "": ["com.example.todo-react-actyx-v1-v2/", "rw"],
                        // map converted-from:x tags into a common namespace so that converters can collaborate
                        "converted-from:" : ["converters/converted-from:", "rw"],
                    }
                },
                // back-converter from v2 to v1
                "com.example.todo-react-actyx-v2-v1": {
                    "": {
                        // from is mounted readonly
                        "from/": ["com.example.todo-react-actyx-v2/", "r"],
                        // to is mounted rw
                        "to/": ["com.example.todo-react-actyx-v1/", "rw"],
                        // the converter has its own mount to store progress info
                        "": ["com.example.todo-react-actyx-v2-v1/", "rw"],
                        // map converted-from:x tags into a common namespace so that converters can collaborate
                        "converted-from:" : ["converters/converted-from:", "rw"],
                    }
                },
                // bridge from actyx todo to jira todo app
                "com.example.bridge-actyx-v1-jira-v1": {
                    "": {
                        // from is mounted readonly
                        "from/": ["com.example.todo-react-actyx-v1/", "r"],
                        // to is mounted rw
                        "to/": ["com.example.todo-react-jira-v1/", "rw"],
                        // the converter has its own mount to store progress info
                        "": ["com.example.bridge-actyx-v1-jira-v1/", "rw"],
                        // map converted-from:x tags into a common namespace so that converters can collaborate
                        "converted-from:" : ["converters/converted-from:", "rw"],
                    }
                },
                // bridge from jira todo to actyx todo app
                "com.example.bridge-jira-v1-actyx-v1": {
                    "": {
                        // from is mounted readonly
                        "from/": ["com.example.todo-react-jira-v1/", "r"],
                        // to is mounted rw
                        "to/": ["com.example.todo-react-actyx-v1/", "rw"],
                        // the converter has its own mount to store progress info
                        "": ["com.example.bridge-jira-v1-actyx-v1/", "rw"],
                        // map converted-from:x tags into a common namespace so that converters can collaborate
                        "converted-from:" : ["converters/converted-from:", "rw"],
                    }
                },
                // the actyx todo app
                "com.example.todo-react-actyx": {
                    // versions starting with 1. are mapped to -v1/
                    "1.": {
                        "": ["com.example.todo-react-actyx-v1/", "rw"],
                    },
                    // versions starting with 2. are mapped to -v2/
                    "2.": {
                        "": ["com.example.todo-react-actyx-v2/", "rw"],
                    }
                },
                // a third party todo app
                "com.example.todo-react-jira": {
                    // versions starting with 1. are mapped to -v1/
                    "1.": {
                        "": ["com.example.todo-react-jira-v1/", "rw"],
                    },
                },
                // the cli
                "com.actyx.cli": {
                    // root for all versions
                    "": {
                        "": ["", "rw"],
                    }
                }
            }
        }
    ).unwrap();
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    fn print_mapping(mapper: &TagMapper, app_id: &AppId, app_version: &str, tag: &str) {
        let global = mapper.to_global(app_id, app_version, tag);
        if let Some(global) = global {
            let local = mapper.to_local(app_id, app_version, &global);
            assert_eq!(local, Some(tag.to_owned()));
            println!("{}: {} <=> {}", app_id, tag, global);
            for other_app_id in mapper.to_local.keys().filter(|id| *id != app_id) {
                if let Some(other_local) = mapper.to_local(other_app_id, app_version, &global) {
                    println!(" {}: {} <=> {}", other_app_id, global, other_local);
                }
            }
        } else {
            println!("{}: {} has no mapping", app_id, tag);
        }
        println!();
    }

    #[test]
    fn parse_json_extended() -> anyhow::Result<()> {
        let mapping = json! {
            {
                "converter-v1-v2": {
                    "from/": ["app1/", "r"],
                    "to/": ["app2/", "rw"],
                    "converter-v1-v2": ["", "rw"],
                },
            }
        };
        let _mapper = TagMapper::parse_extended_json(mapping)?;
        Ok(())
    }

    #[test]
    fn smoke() -> anyhow::Result<()> {
        let mapping = json! {
            {
                "app1": {
                    "": {
                        "": "app1/",
                        "order": "iso-production-order"
                    }
                },
                "app2": {
                    "": {
                        "": "app2/",
                        "order": "iso-production-order"
                    }
                }
            }
        };
        let app1 = AppId::try_from("app1")?;
        let app2 = AppId::try_from("app2")?;
        let mapper = TagMapper::parse_json(mapping)?;
        print_mapping(&mapper, &app1, "", "order");
        print_mapping(&mapper, &app1, "", "order/1234");
        print_mapping(&mapper, &app1, "", "article");
        print_mapping(&mapper, &app1, "", "article/1234");

        print_mapping(&mapper, &app2, "", "order");
        print_mapping(&mapper, &app2, "", "order/1234");
        print_mapping(&mapper, &app2, "", "article");
        print_mapping(&mapper, &app2, "", "article/1234");
        Ok(())
    }
}
