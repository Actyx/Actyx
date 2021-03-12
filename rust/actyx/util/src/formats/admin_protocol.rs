use super::{ActyxOSResult, LogEvent};
use actyxos_sdk::tagged::NodeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AdminProtocol();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogQueryMode {
    All,
    MostRecent {
        count: usize,
    },
    ByTime {
        since: DateTime<Utc>,
        to: Option<DateTime<Utc>>, // None eq. now if follow == false
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQuery {
    pub mode: LogQueryMode,
    pub follow: bool,
}

impl libp2p_streaming_response::Codec for AdminProtocol {
    type Request = AdminRequest;
    type Response = ActyxOSResult<AdminResponse>;
    fn protocol_info() -> &'static [u8] {
        b"/actyx/admin/1.0.0"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    NodesLs,
    // TODO
    //    AppsToken {
    //        app_id: AppId,
    //    },
    SettingsGet {
        scope: axossettings::Scope,
        no_defaults: bool,
    },
    SettingsSet {
        scope: axossettings::Scope,
        json: serde_json::Value,
        ignore_errors: bool,
    },
    SettingsSchema {
        scope: axossettings::Scope,
    },
    SettingsScopes,
    SettingsUnset {
        scope: axossettings::Scope,
    },
    Internal(InternalRequest),
    Logs(LogQuery),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminResponse {
    NodesLsResponse(NodesLsResponse),
    // AppsTokenResponse(String),
    LogsTailResponse,
    SettingsGetResponse(serde_json::Value),
    SettingsSetResponse(serde_json::Value),
    SettingsSchemaResponse(serde_json::Value),
    SettingsScopesResponse(Vec<String>),
    SettingsUnsetResponse,
    Internal(InternalResponse),
    Logs(Vec<LogEvent>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Internal requests, subject to change, undocumented, use at your own risk
pub enum InternalRequest {
    GetSwarmState,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Internal requests, subject to change, undocumented, use at your own risk
pub enum InternalResponse {
    GetSwarmStateResponse(serde_json::Value),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodesLsResponse {
    pub node_id: NodeId,
    pub display_name: String,
    pub started_iso: String,
    pub started_unix: i64,
    pub version: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SetSettingsRequest {
    pub settings: serde_json::Value,
}
