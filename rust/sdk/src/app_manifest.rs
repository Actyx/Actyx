use crate::{app_id, AppId};
use core::convert::TryFrom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestIo {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl From<AppManifest> for AppManifestIo {
    fn from(value: AppManifest) -> Self {
        value.0
    }
}

impl TryFrom<AppManifestIo> for AppManifest {
    type Error = String;
    fn try_from(value: AppManifestIo) -> Result<Self, Self::Error> {
        if value.signature.is_none() && !value.app_id.starts_with("com.example.") {
            Err(format!(
                "Trial app id needs to start with 'com.example.'. Got '{}'.",
                value.app_id
            ))
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(into = "AppManifestIo", try_from = "AppManifestIo")]
pub struct AppManifest(AppManifestIo);

impl AppManifest {
    pub fn signed(app_id: AppId, display_name: String, version: String, signature: String) -> Self {
        Self(AppManifestIo {
            app_id,
            display_name,
            version,
            signature: Some(signature),
        })
    }

    pub fn trial(app_id: AppId, display_name: String, version: String) -> anyhow::Result<Self> {
        if !app_id.starts_with("com.example.") {
            anyhow::bail!("Trial app id needs to start with 'com.example.'. Got '{}'.", app_id);
        }
        Ok(Self(AppManifestIo {
            app_id,
            display_name,
            version,
            signature: None,
        }))
    }

    pub fn app_id(&self) -> AppId {
        self.0.app_id.clone()
    }

    pub fn display_name(&self) -> &str {
        &self.0.display_name
    }

    pub fn version(&self) -> &str {
        &self.0.version
    }

    pub fn signature(&self) -> &Option<String> {
        &self.0.signature
    }

    pub fn is_signed(&self) -> bool {
        self.0.signature.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::app_id;

    #[test]
    fn should_fail_on_non_example_trial() {
        let err =
            AppManifest::trial(app_id!("com.actyx.test-app"), "display name".into(), "v0.0.1".into()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Trial app id needs to start with 'com.example.'. Got 'com.actyx.test-app'."
        );
    }

    #[test]
    fn should_fail_on_non_example_trial_from_io() {
        let err = TryInto::<AppManifest>::try_into(AppManifestIo {
            app_id: app_id!("com.actyx.test-app"),
            display_name: "display name".into(),
            version: "v0.0.1".into(),
            signature: None,
        })
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Trial app id needs to start with 'com.example.'. Got 'com.actyx.test-app'."
        );
    }

    #[test]
    fn should_succeed_creating_and_serializing_trial_manifest() {
        let manifest =
            AppManifest::trial(app_id!("com.example.test-app"), "display name".into(), "v0.0.1".into()).unwrap();
        let serialized = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized,
            r#"{"appId":"com.example.test-app","displayName":"display name","version":"v0.0.1"}"#
        )
    }

    #[test]
    fn serialize() {
        let serialized = serde_json::to_value(AppManifest::signed(
            app_id!("com.not-example.x"),
            "display_name".into(),
            "0.1.0".into(),
            "signature".into(),
        ))
        .unwrap();

        let json = serde_json::json!({
           "appId": "com.not-example.x",
           "displayName": "display_name",
           "version": "0.1.0",
           "signature": "signature"
        });

        println!("serialized {:?}", serialized);
        println!("json {:?}", json);

        assert_eq!(serialized, json);
    }

    #[test]
    fn deserialize_and_eq() {
        let from_json = serde_json::from_value::<AppManifest>(serde_json::json!({
            "appId":"com.actyx.test-app",
            "displayName":"display name",
            "version":"version 0",
            "signature":"v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYWWNuNmlpWXllcFBKeEN6L3hEY3JkQklHcGlxYVRkQVBaakVuRnIzL2xBdGgzaXRzNU9PaTFORjU4M2xFcTJuSVJwOWtIZ1BFSTViTFNPQlFxN2lNQkE9PWlkZXZQdWJrZXl4LTBzaUdHN0dYSGpGaG5oRldya3RiaVZ2Vjgyb1dxTUVzdDBiOVVjWjZYRWd3PWphcHBEb21haW5zgWtjb20uYWN0eXguKmtheFNpZ25hdHVyZXhYekpYa0VkL1BnWjdkcEUzZDVDc0JSaWJHVjBRcE9ZcEhHa3dmV1JEVFNuclk1d25tWDN6YnhMNjA1TkdjK3huTnpKeHoyamp3N1VFemNPTlBrRXN0Q3c9Pf8="
        })).unwrap();
        let manifest = AppManifest::signed(
            app_id!("com.actyx.test-app"),
            "display name".into(),
            "version 0".into(),
            "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYWWNuNmlpWXllcFBKeEN6L3hEY3JkQklHcGlxYVRkQVBaakVuRnIzL2xBdGgzaXRzNU9PaTFORjU4M2xFcTJuSVJwOWtIZ1BFSTViTFNPQlFxN2lNQkE9PWlkZXZQdWJrZXl4LTBzaUdHN0dYSGpGaG5oRldya3RiaVZ2Vjgyb1dxTUVzdDBiOVVjWjZYRWd3PWphcHBEb21haW5zgWtjb20uYWN0eXguKmtheFNpZ25hdHVyZXhYekpYa0VkL1BnWjdkcEUzZDVDc0JSaWJHVjBRcE9ZcEhHa3dmV1JEVFNuclk1d25tWDN6YnhMNjA1TkdjK3huTnpKeHoyamp3N1VFemNPTlBrRXN0Q3c9Pf8=".into(),
        );
        assert_eq!(manifest, from_json);
    }
}

impl Default for AppManifest {
    /// Returns the default manifest.
    ///
    /// The default manifest has the application ID `com.example.trial`,
    /// the display name "Trial App" and the version "0.0.1".
    /// Since it is a trial application manifest, it does not have a signature.
    fn default() -> Self {
        Self {
            app_id: app_id!("com.example.trial"),
            display_name: "Trial App".to_string(),
            version: "0.0.1".to_string(),
            signature: None,
        }
    }
}
