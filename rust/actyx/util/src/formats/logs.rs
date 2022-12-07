use serde::{Deserialize, Serialize};
use std::fmt;

impl From<&tracing::Level> for LogSeverity {
    fn from(l: &tracing::Level) -> Self {
        use tracing::Level;
        match *l {
            Level::TRACE => LogSeverity::Trace,
            Level::DEBUG => LogSeverity::Debug,
            Level::INFO => LogSeverity::Info,
            Level::WARN => LogSeverity::Warn,
            Level::ERROR => LogSeverity::Error,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(from = "String", into = "String")]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    RustLog(String),
}

impl From<&str> for LogSeverity {
    fn from(other: &str) -> Self {
        let from = other.to_lowercase();
        match from.as_ref() {
            "trace" => LogSeverity::Trace,
            "debug" => LogSeverity::Debug,
            "info" => LogSeverity::Info,
            "warn" => LogSeverity::Warn,
            "error" => LogSeverity::Error,
            _ => LogSeverity::RustLog(from),
        }
    }
}

impl From<String> for LogSeverity {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<LogSeverity> for String {
    fn from(ls: LogSeverity) -> Self {
        ls.to_string()
    }
}

impl Default for LogSeverity {
    fn default() -> Self {
        LogSeverity::Info
    }
}

impl fmt::Display for LogSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LogSeverity::Trace => "TRACE",
            LogSeverity::Debug => "DEBUG",
            LogSeverity::Info => "INFO",
            LogSeverity::Warn => "WARN",
            LogSeverity::Error => "ERROR",
            LogSeverity::RustLog(l) => l.as_str(),
        };
        write!(f, "{}", name)
    }
}

#[test]
fn levels() {
    assert_eq!(serde_json::to_string(&LogSeverity::Error).unwrap(), "\"ERROR\"");
    assert_eq!(serde_json::to_string(&LogSeverity::Warn).unwrap(), "\"WARN\"");
    assert_eq!(serde_json::to_string(&LogSeverity::Info).unwrap(), "\"INFO\"");
    assert_eq!(serde_json::to_string(&LogSeverity::Debug).unwrap(), "\"DEBUG\"");
    assert_eq!(
        serde_json::to_string(&LogSeverity::RustLog("yamux=trace".to_owned())).unwrap(),
        "\"yamux=trace\""
    );

    assert_eq!(
        serde_json::from_str::<LogSeverity>("\"ERROR\"").unwrap(),
        LogSeverity::Error
    );
    assert_eq!(
        serde_json::from_str::<LogSeverity>("\"WARN\"").unwrap(),
        LogSeverity::Warn
    );
    assert_eq!(
        serde_json::from_str::<LogSeverity>("\"INFO\"").unwrap(),
        LogSeverity::Info
    );
    assert_eq!(
        serde_json::from_str::<LogSeverity>("\"DEBUG\"").unwrap(),
        LogSeverity::Debug
    );
    assert_eq!(
        serde_json::from_str::<LogSeverity>("\"yamux=trace\"").unwrap(),
        LogSeverity::RustLog("yamux=trace".to_owned())
    );
}
