use std::str::FromStr;

use actyx_sdk::AppId;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

fn deserialize_trial_app_id<'de, D: Deserializer<'de>>(d: D) -> Result<AppId, D::Error> {
    let s = <String>::deserialize(d)?;
    let app_id = AppId::from_str(&s).map_err(D::Error::custom)?;
    TrialAppManifest::validate_app_id(&app_id).map_err(D::Error::custom)?;
    Ok(app_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialAppManifest {
    #[serde(deserialize_with = "deserialize_trial_app_id")]
    app_id: AppId,
    pub display_name: String,
    pub version: String,
}

impl TrialAppManifest {
    pub fn new(app_id: AppId, display_name: String, version: String) -> anyhow::Result<Self> {
        TrialAppManifest::validate_app_id(&app_id)?;
        Ok(TrialAppManifest {
            app_id,
            display_name,
            version,
        })
    }

    pub fn get_app_id(&self) -> AppId {
        self.app_id.clone()
    }

    fn validate_app_id(app_id: &AppId) -> anyhow::Result<()> {
        if app_id.starts_with("com.example.") {
            Ok(())
        } else {
            anyhow::bail!("Trial app id needs to start with 'com.example.'. Got '{}'.", app_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::app_id;

    use super::TrialAppManifest;

    #[test]
    fn should_succeed_creating_and_serializing_manifest() {
        let manifest =
            TrialAppManifest::new(app_id!("com.example.test-app"), "display name".into(), "v0.0.1".into()).unwrap();
        let serialized = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized,
            r#"{"appId":"com.example.test-app","displayName":"display name","version":"v0.0.1"}"#
        )
    }

    #[test]
    fn should_fail_creating_manifest() {
        let _ =
            TrialAppManifest::new(app_id!("com.actyx.test-app"), "display name".into(), "v0.0.1".into()).unwrap_err();
    }

    #[test]
    fn should_succeed_deserializing_manifest() {
        let serialized = r#"{"appId":"com.example.test-app","displayName":"display name","version":"v0.0.1"}"#;
        let _: TrialAppManifest = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn should_fail_deserializing_manifest() {
        let serialized = r#"{"appId":"com.actyx.test-app","displayName":"display name","version":"v0.0.1"}"#;
        let result = serde_json::from_str::<TrialAppManifest>(&serialized).unwrap_err();
        assert_eq!(
            result.to_string(),
            "Trial app id needs to start with \'com.example.\'. Got \'com.actyx.test-app\'. at line 1 column 29"
        )
    }
}
