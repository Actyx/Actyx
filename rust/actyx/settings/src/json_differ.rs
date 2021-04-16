use crate::Scope;
use serde_json::Value;
use std::{collections::BTreeSet, convert::TryInto};

pub struct JsonDiffer {
    pub changed_scopes: BTreeSet<Scope>,
    scope: Scope,
}

impl JsonDiffer {
    pub fn new() -> Self {
        JsonDiffer {
            changed_scopes: BTreeSet::new(),
            scope: Scope::root(),
        }
    }
}

impl Default for JsonDiffer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> treediff::Delegate<'a, treediff::value::Key, Value> for JsonDiffer {
    fn push(&mut self, key: &treediff::value::Key) {
        match key {
            treediff::value::Key::String(key) => {
                let key_scope = key.clone().try_into().unwrap();
                self.scope = self.scope.append(&key_scope);
            }
            // Array changed; we don't care about the changed position
            treediff::value::Key::Index(_) => {}
        }
    }
    fn pop(&mut self) {
        self.scope.pop_mut();
    }

    fn removed<'b>(&mut self, k: &'b treediff::value::Key, _v: &'a Value) {
        match k {
            treediff::value::Key::String(key) => {
                let ptr = self.scope.clone();
                let key_scope = key.clone().try_into().unwrap();
                self.scope = self.scope.append(&key_scope);
                let scope = std::mem::replace(&mut self.scope, ptr);
                self.changed_scopes.insert(scope);
            }
            treediff::value::Key::Index(_) => {
                // Array changed, we don't care about the changed position
                self.changed_scopes.insert(self.scope.clone());
            }
        }
    }

    fn added(&mut self, k: &treediff::value::Key, _v: &Value) {
        self.removed(k, _v)
    }

    fn modified(&mut self, _old: &'a Value, _new: &'a Value) {
        self.changed_scopes.insert(self.scope.clone());
    }
}
