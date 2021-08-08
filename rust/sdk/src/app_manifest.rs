use crate::AppId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
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
