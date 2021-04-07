use tokio::sync::oneshot::Sender;
use util::formats::ActyxOSResult;

pub const SYSTEM_SCOPE: &str = "com.actyx";

pub type SettingsResponse<T> = ActyxOSResult<T>;

#[derive(Debug)]
pub enum SettingsRequest {
    GetSettings {
        scope: axossettings::Scope,
        no_defaults: bool,
        response: Sender<SettingsResponse<serde_json::Value>>,
    },
    SetSettings {
        scope: axossettings::Scope,
        json: serde_json::Value,
        response: Sender<SettingsResponse<serde_json::Value>>,
        ignore_errors: bool,
    },
    UnsetSettings {
        scope: axossettings::Scope,
        response: Sender<SettingsResponse<()>>,
    },
    SetSchema {
        scope: axossettings::Scope,
        json: serde_json::Value,
        response: Sender<SettingsResponse<()>>,
    },
    DeleteSchema {
        scope: axossettings::Scope,
        response: Sender<SettingsResponse<()>>,
    },
    GetSchemaScopes {
        response: Sender<SettingsResponse<Vec<String>>>,
    },
    GetSchema {
        scope: axossettings::Scope,
        response: Sender<SettingsResponse<serde_json::Value>>,
    },
}
