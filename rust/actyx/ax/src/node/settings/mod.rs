use tokio::sync::oneshot::Sender;
use util::formats::ActyxOSResult;

pub const SYSTEM_SCOPE: &str = "com.actyx";
pub fn system_scope() -> settings::Scope {
    SYSTEM_SCOPE.parse().unwrap()
}
pub fn is_system_scope(scope: &settings::Scope) -> bool {
    scope.first() == Some(SYSTEM_SCOPE.to_string())
}

pub type SettingsResponse<T> = ActyxOSResult<T>;

#[derive(Debug)]
pub enum SettingsRequest {
    GetSettings {
        scope: settings::Scope,
        no_defaults: bool,
        response: Sender<SettingsResponse<serde_json::Value>>,
    },
    SetSettings {
        scope: settings::Scope,
        json: serde_json::Value,
        response: Sender<SettingsResponse<serde_json::Value>>,
        ignore_errors: bool,
    },
    UnsetSettings {
        scope: settings::Scope,
        response: Sender<SettingsResponse<()>>,
    },
    SetSchema {
        scope: settings::Scope,
        json: serde_json::Value,
        response: Sender<SettingsResponse<()>>,
    },
    DeleteSchema {
        scope: settings::Scope,
        response: Sender<SettingsResponse<()>>,
    },
    GetSchemaScopes {
        response: Sender<SettingsResponse<Vec<String>>>,
    },
    GetSchema {
        scope: settings::Scope,
        response: Sender<SettingsResponse<serde_json::Value>>,
    },
}
