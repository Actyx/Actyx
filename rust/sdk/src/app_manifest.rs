use crate::{app_id, AppId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
/// App manifest used for requesting a bearer token via the API. For more information see the
/// [docs](https://developer.actyx.com/docs/how-to/app-auth/authenticate-with-app-manifest).
pub struct AppManifest {
    /// App id in lower case and in reverse domain name notation
    pub app_id: AppId,
    /// Display name of the app
    pub display_name: String,
    /// Version string of the app
    pub version: String,
    /// Developer certificate's signature
    pub signature: Option<String>,
}

impl AppManifest {
    pub fn new(app_id: AppId, display_name: String, version: String, signature: Option<String>) -> Self {
        AppManifest {
            app_id,
            display_name,
            version,
            signature,
        }
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
