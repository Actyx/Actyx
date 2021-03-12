//! #Monitoring formats
//!
//! This module contains the formats that are sent on the IPFS monitoring pubsub topic of an installation.
use actyxos_sdk::event::SourceId;
use chrono::prelude::*;
use libipld::cid::Cid;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
/// Enumeration of the possible event types that can appear on the IPFS monitoring topic of an installation.
pub enum MonitoringMessage {
    Log(Vec<PublishLog>),
    Alert(PublishAlert),
    Heartbeat(PublishHeartbeat),
    Meta(PublishMeta),
    Fetch(FetchResponse),
    Identify(PublishIdentifyResponse),
    RunStats(PublishRunStats),
    Backup(BackupResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FullMonitoringMessage {
    Log(Vec<PublishLog>),
    Other(MonitoringMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishIdentifyResponse {
    pub source: SourceId,
    // todo
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    pub source: SourceId,
    pub url: String,
    pub response: serde_json::Value,
    pub delay_us: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatsValues {
    pub counters: Value,
    pub durations: Value,
    pub gauges: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRunStats {
    pub source: SourceId,
    pub counters: Value,
    pub durations: Value,
    pub gauges: Value,
}

impl PublishRunStats {
    pub fn new(values: RunStatsValues, source_id: SourceId) -> Self {
        PublishRunStats {
            source: source_id,
            counters: values.counters,
            durations: values.durations,
            gauges: values.gauges,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishHeartbeat {
    pub source: SourceId,
    pub time: u64,
}

impl PublishHeartbeat {
    pub fn new(time: u64, source_id: SourceId) -> Self {
        PublishHeartbeat {
            source: source_id,
            time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishAlert {
    pub source: SourceId,
    pub time: u64,
    pub message: String,
}

impl PublishAlert {
    pub fn new(time: u64, message: String, source_id: SourceId) -> Self {
        PublishAlert {
            source: source_id,
            time,
            message,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishMeta {
    pub source: SourceId,
    pub message: String,
}

impl PublishMeta {
    pub fn new(message: String, source_id: SourceId) -> Self {
        PublishMeta {
            source: source_id,
            message,
        }
    }
}

// [
//     {
//         "type":"log",
//         "sourceId":"hwUH4R8Y1wx",
//         "level":"INFO",
//         "message":"WebView: executing identify (from http://localhost:9090/ipfs/QmY95r5w5ynVuSpM5zW1XKJijTZM7BdR7zcbmDRrASSUtZ/chrome/app.6d4636074a3de0fca029.js)",
//         "serialNumber":"BR91203512",
//         "tag":"io.actyx.shell.activity.MainActivity",
//         "timestamp":"2019-10-01T14:23:26.041Z"
//     }
// ]

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishLog {
    pub source_id: SourceId,
    level: String,
    message: String,
    pub serial_number: String,
    tag: String,
    timestamp: DateTime<Utc>,
    r#type: String,
}

impl PublishLog {
    pub fn new(
        level: String,
        message: String,
        serial_number: String,
        tag: String,
        timestamp: DateTime<Utc>,
        source_id: SourceId,
    ) -> Self {
        PublishLog {
            source_id,
            level,
            message,
            serial_number,
            tag,
            timestamp,
            r#type: "log".to_string(),
        }
    }

    pub fn source_id(&self) -> SourceId {
        self.source_id
    }
    pub fn level(&self) -> &str {
        self.level.as_str()
    }
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
    pub fn serial_number(&self) -> &str {
        self.serial_number.as_str()
    }
    pub fn tag(&self) -> &str {
        self.tag.as_str()
    }
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

use crate::wrapping_subscriber::ConvertEvent;
use std::fmt::{self, Write};
use std::str::FromStr;
use tracing::field::{Field, Visit};
use tracing::Event;

pub struct MessageVisitor<'a> {
    pub string: &'a mut String,
}

impl<'a> Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            write!(self.string, "{:?}; ", value).unwrap();
        }
    }
}

impl ConvertEvent for PublishLog {
    fn convert(ev: &Event<'_>) -> Self {
        let metadata = ev.metadata();
        let level = metadata.level().to_string();
        let mut message = String::from("");
        let mut visitor = MessageVisitor { string: &mut message };
        ev.record(&mut visitor);
        let tag = metadata.target().to_string();

        PublishLog::new(
            level,
            message,
            "".to_owned(),
            tag,
            Utc::now(),
            SourceId::from_str("dummy").unwrap(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResponse {
    pub source_id: SourceId,
    pub hash: Cid,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_monitoring_message(json: &str) {
        let _: FullMonitoringMessage = serde_json::from_str(json).expect("can not deserialize log message");
    }

    #[test]
    fn monitoring_protocol_test() {
        check_monitoring_message(
            r#"[{"type":"log","level":"INFO","message":"monitorTask run(); IpfsService.State: RUNNING","serialNumber":"BR81200790","sourceId":"sehM58jc6XP","tag":"io.actyx.shell.service.IpfsService","timestamp":"2019-10-16T15:30:07.037Z"}]"#,
        );
        check_monitoring_message(r#"{"type":"heartbeat","source":"HJTFA1FkkY3","time":15712398000000}"#);
        check_monitoring_message(
            r#"{"type":"runStats","source":"RjxhoJsRYnG","counters":{"cache-get/blockList-memo":[47297,63],"cache-get/consnode-memo":[8814,5],"cache-hit/blockList-memo":[39607,58],"snapshot-wanted/edge.ax.sf.ProcessRegistry":[5,0],"snapshot-found/edge.ax.sf.ProcessRegistry":[5,0],"cache-get/events-memo":[47076,54],"cache-hit/events-memo":[41929,51],"snapshot-wanted/edge.ax.sf.AtomRegistry":[1,0],"snapshot-found/edge.ax.sf.AtomRegistry":[1,0],"snapshot-wanted/MachineOverviewFish":[1,0],"snapshot-found/MachineOverviewFish":[1,0],"snapshot-wanted/edge.ax.sf.ProcessExecution":[23,0],"snapshot-found/edge.ax.sf.ProcessExecution":[15,0],"snapshot-stored/edge.ax.sf.ProcessExecution":[117,0],"snapshot-stored/edge.ax.sf.ProcessRegistry":[23,0],"errprof-fetch-limiter":[64,0],"errprof-store-ingestion-blocks":[8,0],"cache-hit/consnode-memo":[1164,0],"snapshot-stored/edge.ax.sf.AtomRegistry":[8,0],"snapshot-stored/MachineOverviewFish":[24,0]},"durations":{"fetch-limiter":{"count":27,"pending":1,"discarded":0,"min":9000,"median":52000,"_90":108000,"_95":118000,"_99":131000,"max":131000},"inject-events/edge.ax.sf.ProcessRegistry":{"count":4,"pending":0,"discarded":0,"min":0,"median":0,"_90":1000,"_95":1000,"_99":1000,"max":1000},"inject-compute/edge.ax.sf.ProcessRegistry":{"count":4,"pending":0,"discarded":0,"min":2000,"median":6000,"_90":6000,"_95":6000,"_99":6000,"max":6000},"inject-events/edge.ax.sf.ProcessExecution":{"count":44,"pending":0,"discarded":0,"min":0,"median":0,"_90":1000,"_95":1000,"_99":1000,"max":1000},"inject-compute/edge.ax.sf.ProcessExecution":{"count":44,"pending":0,"discarded":0,"min":1000,"median":1000,"_90":4000,"_95":5000,"_99":6000,"max":6000},"store-ingestion-blocks":{"count":3,"pending":0,"discarded":0,"min":44000,"median":50000,"_90":56000,"_95":56000,"_99":56000,"max":56000},"store-decompress":{"count":3,"pending":0,"discarded":0,"min":2000,"median":3000,"_90":6000,"_95":6000,"_99":6000,"max":6000}},"gauges":{"cache-max/consnode-memo":{"last":10000,"max":10000},"cache-max/events-memo":{"last":100000,"max":100000},"cache-max/blockList-memo":{"last":5000,"max":5000},"cache-size/blockList-memo":{"last":5000,"max":5000},"cache-size/consnode-memo":{"last":7594,"max":7594},"cache-size/events-memo":{"last":96573,"max":100000},"memory.usedJSHeapSize":{"last":123000000,"max":167000000},"memory.totalJSHeapSize":{"last":139000000,"max":269000000},"memory.jsHeapSizeLimit":{"last":521000000,"max":521000000},"prf-procs":{"last":754,"max":754}}}"#,
        );
        check_monitoring_message(
            r#"{"type":"identify","source":"GUTrt5vwSVu","deviceInfo":{"type":"android","app":{"firstInstallTime":1566481430193,"lastUpdateTime":1567674680010,"versionCode":56,"versionName":"1.11.6"},"battery":{"currentAverage":0,"currentNow":0,"level":100},"disk":{"data":{"free":19359858688,"path":"/data","total":23642730496},"root":{"free":2353008640,"path":"/system","total":4127625216},"ext":{"free":19359858688,"path":"/storage/emulated/0","total":23642730496}},"memory":{"available":668917760,"lowMemory":false,"threshold":150994944,"total":1955307520},"network":{"fequency":5220,"ipAddress":-842523988,"rssi":-45,"ssid":"\"CTA-BDE\""},"serialNumber":"BR84400534","uptimeMillis":1453663030,"webView":{"userAgent":"Mozilla/5.0 (Linux; Android 6.0.1; ET5X Build/03-21-20-MG-0R-M1-U00-STD; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/77.0.3865.116 Safari/537.36"},"config":{"factory":"cta","deviceEndpoint":{"host":"localhost:8080/ws","protocol":"ws"},"userIdMappingService":{"uri":"http://172.26.100.22/rfid/uid","user":"cta","password":"iAmC9E0tNof7E7YNv8fvEWc7DUa_yLCc"},"ipfs":{"gossipPeriodMs":30000,"pubSubMode":"ReadWrite","topic":"/cta/prod/2019-08-07/2","monitoringTopic":"/cta/prod/2019-08-07/monitoring","rootValidationTimeout":120000},"plugin":{"activities":{"cutOffDays":28}},"initializeTerminalFish":{"tags":[{"semantics":"WorkstationType","value":"Machine"},{"semantics":"Workstation","value":"CFFILL"}]}},"deviceConfig":{"config":{"factory":"cta","deviceEndpoint":{"host":"localhost:8080/ws","protocol":"ws"},"userIdMappingService":{"uri":"http://172.26.100.22/rfid/uid","user":"cta","password":"iAmC9E0tNof7E7YNv8fvEWc7DUa_yLCc"},"ipfs":{"gossipPeriodMs":30000,"pubSubMode":"ReadWrite","topic":"/cta/prod/2019-08-07/2","monitoringTopic":"/cta/prod/2019-08-07/monitoring","rootValidationTimeout":120000},"plugin":{"activities":{"cutOffDays":28}},"initializeTerminalFish":{"tags":[{"semantics":"WorkstationType","value":"Machine"},{"semantics":"Workstation","value":"CFFILL"}]}},"forceDeviceConfigUpdate":true,"ipfs":{"monitoringTopic":"/cta/prod/2019-08-07/monitoring","topic":"/cta/prod/2019-08-07/2"},"root":"/ipfs/QmczNoFZzbQQTYFR85CnTMCbrh494ohmzfxBV2qWExuCGK/chrome/"}},"timestamp":1571241016672000}"#,
        );
        check_monitoring_message(r#"{"type":"meta","source":"KVqhXEqWr7S","message":"executing identify"}"#)
    }
}
