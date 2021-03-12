use crate::{json_differ::JsonDiffer, scope::Scope};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Value {value} at path {path} is not of type object or array.")]
    NotAnObjectOrArray { path: Scope, value: serde_json::Value },
    #[error("Index {index} invalid at path {path}.")]
    InvalidArrayIndex { path: Scope, index: usize },
}

type Result<T> = std::result::Result<T, Error>;

pub trait JsonValue {
    /// Given two JSON objects, will return a set of changed scopes.
    fn diff(&self, right: &Self) -> BTreeSet<Scope>;

    /// Removes the value at `scope`.
    fn remove_at(&self, scope: &Scope) -> serde_json::Value;

    /// Updates the value at `scope` and creates empty objects or arrays at intermediate levels.
    /// Path elements of type number are used to index into arrays which are created if they do not
    /// already exist and the index is zero.
    /// Will fail with `NotAnObjectOrArray` if `value` already contains a simple JSON value at any
    /// parent level of `scope`.
    fn update_at(&self, scope: &Scope, value: serde_json::Value) -> Result<serde_json::Value>;

    /// Updates the value at `scope` and creates empty objects or arrays at intermediate levels.
    /// Path elements of type number are used to index into arrays which are created if they do not
    /// already exist and the index is zero.
    /// Intermediate simple values will be silently replaced by objects or arrays.
    fn update_at_force(&self, scope: &Scope, value: serde_json::Value) -> serde_json::Value;
}

/// Makes sure `scope` exists within this value by creating empty objects or arrays at every
/// level and a `null` value at the leaf if no value already exists.
/// Path elements of type number are used to index into arrays which are created if they do not
/// already exist and the index is zero.
/// If `force` is set to `true` pre-existing simple non-leaf values are silently replaced by
/// objects or arrays. Otherwise `Error::NotAnObjectOrArray` is returned.
fn mk_path(value: Option<&serde_json::Value>, scope: &Scope, prefix: Scope, force: bool) -> Result<serde_json::Value> {
    match scope.split_first() {
        None => Ok(value.cloned().unwrap_or(serde_json::Value::Null)),
        Some((head, tail)) => {
            let next_prefix = prefix.append(&head);
            let key = head.to_string();
            let idx: Option<usize> = key.as_str().parse().ok();
            match (value, idx, force) {
                (Some(serde_json::Value::Array(arr)), Some(idx), _) => {
                    if idx < arr.len() {
                        // replace
                        let mut arr = arr.clone();
                        arr[idx] = mk_path(arr.get(idx), &tail, next_prefix, force)?;
                        Ok(serde_json::Value::Array(arr))
                    } else if idx == arr.len() || force {
                        // append
                        let mut arr = arr.clone();
                        arr.push(mk_path(None, &tail, next_prefix, force)?);
                        Ok(serde_json::Value::Array(arr))
                    } else {
                        Err(Error::InvalidArrayIndex {
                            index: idx,
                            path: prefix,
                        })
                    }
                }
                (Some(serde_json::Value::Object(obj)), _, _) => {
                    let mut obj = obj.clone();
                    obj.insert(key.clone(), mk_path(obj.get(&key), &tail, next_prefix, force)?);
                    Ok(serde_json::Value::Object(obj))
                }
                (None, Some(idx), _) | (Some(_), Some(idx), true) => {
                    if idx == 0 || force {
                        // create array if missing or force overwrite
                        let arr = vec![mk_path(None, &tail, next_prefix, force)?];
                        Ok(serde_json::Value::Array(arr))
                    } else {
                        Err(Error::InvalidArrayIndex {
                            index: idx,
                            path: prefix,
                        })
                    }
                }
                (None, None, _) | (Some(_), None, true) => {
                    // create object if missing or force overwrite
                    let mut obj = serde_json::Map::new();
                    obj.insert(key.clone(), mk_path(obj.get(&key), &tail, next_prefix, true)?);
                    Ok(serde_json::Value::Object(obj))
                }
                (Some(value), _, _) => Err(Error::NotAnObjectOrArray {
                    path: prefix,
                    value: value.clone(),
                }),
            }
        }
    }
}

fn update_at(obj: &mut serde_json::Value, scope: &Scope, value: serde_json::Value) {
    if let Some(v) = obj.pointer_mut(scope.as_json_ptr().as_str()) {
        // errors are ignored so we return the unmodified object
        *v = value
    };
}

impl JsonValue for serde_json::Value {
    fn diff(&self, right: &Self) -> BTreeSet<Scope> {
        let mut differ = JsonDiffer::new();
        treediff::diff(self, right, &mut differ);
        differ.changed_scopes
    }

    fn remove_at(&self, scope: &Scope) -> Self {
        if let Some((init, last)) = scope.split_last() {
            let mut obj = self.clone();
            if let Some(parent) = obj.pointer_mut(init.as_json_ptr().as_str()) {
                // errors are ignored so we return the unmodified object
                match parent {
                    serde_json::Value::Object(obj) => {
                        obj.remove(&last.to_string());
                    }
                    serde_json::Value::Array(arr) => {
                        if let Ok(idx) = last.to_string().as_str().parse::<usize>() {
                            if idx < arr.len() {
                                arr.remove(idx);
                            }
                        }
                    }
                    _ => {}
                }
            }
            obj
        } else {
            json!({})
        }
    }

    fn update_at(&self, scope: &Scope, value: Self) -> Result<Self> {
        mk_path(Some(self), scope, Scope::root(), false).map(|mut obj| {
            update_at(&mut obj, scope, value);
            obj
        })
    }

    fn update_at_force(&self, scope: &Scope, value: serde_json::Value) -> Self {
        let mut obj = mk_path(Some(self), scope, Scope::root(), true).unwrap();
        update_at(&mut obj, scope, value);
        obj
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::scope::Scope;
    use serde_json::{json, Value};

    fn mp(value: serde_json::Value, scope: &'static str, force: bool) -> Result<serde_json::Value> {
        super::mk_path(Some(&value), &scope.into(), Scope::root(), force)
    }

    #[test]
    fn mk_path() {
        assert_eq!(mp(json!({}), "key", false).unwrap(), json!({ "key": null }));
        assert_eq!(
            mp(json!({}), "key/sub", false).unwrap(),
            json!({ "key": { "sub": null } }),
        );

        assert_eq!(
            mp(json!({ "key": {} }), "key/sub", false).unwrap(),
            json!({ "key": { "sub": null } }),
        );

        assert_eq!(
            mp(json!({ "key": "value" }), "key", false).unwrap(),
            json!({ "key": "value" }),
        );

        assert_eq!(
            mp(json!({ "key": { "sub": {} } }), "key", false).unwrap(),
            json!({ "key": { "sub": {} } }),
        );

        assert_eq!(
            mp(json!({ "key": { "sub": {} } }), "key/sub2/0", false).unwrap(),
            json!({ "key": { "sub": {}, "sub2": [null] } }),
        );

        assert_eq!(
            mp(json!({ "key": { "sub": {} } }), "key/sub/0", false).unwrap(),
            json!({ "key": { "sub": { "0": null } } }),
        );

        assert_eq!(
            format!(
                "{}",
                mp(json!({ "key": { "sub": {} } }), "key/sub2/1", false).unwrap_err()
            ),
            "Index 1 invalid at path key/sub2.",
        );
    }

    #[test]
    fn mk_path_preexisting_simple_values() {
        assert_eq!(
            mp(json!({ "key": "value" }), "key", false).unwrap(),
            json!({ "key": "value" })
        );
    }

    #[test]
    fn mk_path_over_preexisting_simple_value() {
        assert_eq!(
            format!(
                "{}",
                mp(json!({ "key": { "sub": "value" } }), "key/sub/sub", false).unwrap_err()
            ),
            "Value \"value\" at path key/sub is not of type object or array.",
        );
        assert_eq!(
            format!(
                "{}",
                mp(json!({ "key": { "sub": "value" } }), "key/sub/0", false).unwrap_err()
            ),
            "Value \"value\" at path key/sub is not of type object or array.",
        );
    }

    #[test]
    fn mk_path_force_over_preexisting_simple_value() {
        assert_eq!(
            mp(json!({ "key": { "sub": "value" } }), "key/sub/sub", true).unwrap(),
            json!({ "key": { "sub": { "sub": null } } }),
        );
        assert_eq!(
            mp(json!({ "key": { "sub": "value" } }), "key/sub/0", true).unwrap(),
            json!({ "key": { "sub": [null] } }),
        );
    }

    #[test]
    fn mk_path_force_over_nonzero_idx() {
        assert_eq!(
            mp(json!({ "key": { "sub": "value" } }), "key/sub/2", true).unwrap(),
            json!({ "key": { "sub": [null] } }),
        );
    }

    #[test]
    pub fn diff_null() {
        let right = json!({"title": "Hello!"});
        let p = Value::Null.diff(&right);
        assert_eq!(p, maplit::btreeset![Scope::root()]);
    }

    #[test]
    pub fn diff_with_null() {
        let left = json!({"title": "Hello!"});
        let p = left.diff(&Value::Null);
        assert_eq!(p, maplit::btreeset![Scope::root()])
    }

    #[test]
    pub fn diff_array() {
        let left = json!(["hello", "bye"]);
        let right = json!([]);
        let p = left.diff(&right);
        assert_eq!(p, maplit::btreeset![Scope::root()]);

        let left = json!(["hello", "bye", "hi"]);
        let right = json!(["hello"]);
        let p = left.diff(&right);
        assert_eq!(p, maplit::btreeset![Scope::root()]);
    }

    #[test]
    pub fn diff_array_object() {
        let left = json!(["hello", "bye"]);
        let right = json!({"hello": "bye"});
        let p = left.diff(&right);
        assert_eq!(p, maplit::btreeset!["hello".into(), Scope::root()]);
    }

    #[test]
    pub fn diff_nested() {
        let left = json!({
            "root": {
                "sub1":{
                    "prop": "unchanged",
                    "someArray": [],
                    "someOtherArray": []
                },
                "sub2": { "something": { "changed": "here" }}
            }
        });
        let right = json!({
            "root": {
                "sub1":{
                    "prop": "changed",
                    "someArray": ["insert"]
                },
                "sub2": { "something": { "changed": "for real" }}
            }
        });
        let diff = left.diff(&right);
        assert_eq!(
            diff,
            maplit::btreeset![
                "root/sub1/prop".into(),
                "root/sub1/someArray".into(),
                "root/sub1/someOtherArray".into(),
                "root/sub2/something/changed".into(),
            ]
        );
    }

    #[test]
    pub fn remove_at_empty() {
        assert_eq!(json!({}).remove_at(&Scope::root()), json!({}));
        assert_eq!(json!({}).remove_at(&"a/b".into()), json!({}));
        assert_eq!(json!({}).remove_at(&"a/0".into()), json!({}));
    }

    #[test]
    pub fn remove_at() {
        assert_eq!(json!({ "foo": "bar" }).remove_at(&"foo".into()), json!({}));
        assert_eq!(
            json!({ "foo": { "bar": "baz" } }).remove_at(&"foo/bar".into()),
            json!({ "foo": {} }),
        );
        assert_eq!(
            json!({ "foo": { "bar": "baz", "bar2": "baz2" } }).remove_at(&"foo/bar".into()),
            json!({ "foo": { "bar2": "baz2"} }),
        );
        assert_eq!(
            json!({ "foo": ["bar", "baz"] }).remove_at(&"foo/1".into()),
            json!({ "foo": ["bar"] }),
        );
    }

    #[test]
    pub fn remove_at_ignore_nonexisting() {
        assert_eq!(
            json!({ "foo": { "bar": "baz" } }).remove_at(&"foo/0".into()),
            json!({ "foo": { "bar": "baz" } }),
        );
        assert_eq!(
            json!({ "foo": ["bar", "baz"] }).remove_at(&"foo/2".into()),
            json!({ "foo": ["bar", "baz"] }),
        );
        assert_eq!(
            json!({ "foo": ["bar", "baz"] }).remove_at(&"foo/bar".into()),
            json!({ "foo": ["bar", "baz"] }),
        );
    }

    #[test]
    pub fn update_at_create() {
        assert_eq!(
            json!({}).update_at(&"a/b".into(), json!("value")).unwrap(),
            json!({"a": { "b": "value" } }),
        );
        assert_eq!(
            json!({}).update_at(&"a/b/0".into(), json!("value")).unwrap(),
            json!({"a": { "b": ["value"] } }),
        );
        assert_eq!(
            json!({"a": { "b": ["value"] } })
                .update_at(&"a/b/1".into(), json!("value2"))
                .unwrap(),
            json!({"a": { "b": ["value", "value2"] } }),
        );
    }

    #[test]
    pub fn update_at_replace() {
        assert_eq!(
            json!({"a": { "b": "value" } })
                .update_at(&"a/b".into(), json!("updated"))
                .unwrap(),
            json!({"a": { "b": "updated" } }),
        );
        assert_eq!(
            json!({"a": { "b": "value" } })
                .update_at(&"a/b".into(), json!({ "c": "x" }))
                .unwrap(),
            json!({"a": { "b": { "c": "x" } } }),
        );
        assert_eq!(
            json!({"a": { "b": { "c": "x" } } })
                .update_at(&"a/b".into(), json!("value"))
                .unwrap(),
            json!({"a": { "b": "value" } }),
        );
        assert_eq!(
            json!({"a": [{ "b": { "c": "x" } }] })
                .update_at(&"a/0/b".into(), json!("value"))
                .unwrap(),
            json!({"a": [{ "b": "value" }] }),
        );
    }

    #[test]
    pub fn update_at_fail() {
        assert_eq!(
            format!(
                "{}",
                json!({"a": { "b": "x" } })
                    .update_at(&"a/b/c".into(), json!("value"))
                    .unwrap_err()
            ),
            "Value \"x\" at path a/b is not of type object or array.",
        );
        assert_eq!(
            format!(
                "{}",
                json!({"a": [{ "b": "x" }] })
                    .update_at(&"a/2".into(), json!("value"))
                    .unwrap_err()
            ),
            "Index 2 invalid at path a.",
        );
    }

    #[test]
    pub fn update_at_force() {
        assert_eq!(
            json!({"a": { "b": "x" } }).update_at_force(&"a/b/c".into(), json!("value")),
            json!({"a": { "b": { "c": "value" } } }),
        );
    }
}
