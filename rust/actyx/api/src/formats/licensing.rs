use actyx_sdk::AppId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Licensing {
    node: String,
    pub apps: BTreeMap<AppId, String>,
}

impl Licensing {
    pub fn new(node: String, apps: BTreeMap<AppId, String>) -> Self {
        Self { node, apps }
    }

    pub fn is_node_licensed(&self) -> bool {
        self.node != "development"
    }

    pub fn app_id_license(&self, app_id: &AppId) -> Option<&String> {
        self.apps.get(app_id)
    }
}

impl Default for Licensing {
    fn default() -> Self {
        Licensing {
            node: "development".into(),
            apps: BTreeMap::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::formats::Licensing;

    #[test]
    fn default() {
        let licensing = Licensing::default();
        assert_eq!(licensing.node, "development");
        assert!(licensing.apps.is_empty());
    }

    #[test]
    fn is_node_licensed() {
        let licensing = Licensing::default();
        assert!(!licensing.is_node_licensed());

        let licensing = Licensing {
            node: "licensed".into(),
            apps: BTreeMap::default(),
        };
        assert!(licensing.is_node_licensed());
    }
}
