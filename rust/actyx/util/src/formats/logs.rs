use super::{ax_err, ActyxOSCode, ActyxOSError, ActyxOSResult};
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
};

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

#[derive(Serialize, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
impl<'de> serde::de::Deserialize<'de> for LogSeverity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let str = <String>::deserialize(deserializer)?;
        (&*str).try_into().map_err(serde::de::Error::custom)
    }
}
impl TryFrom<&str> for LogSeverity {
    type Error = ActyxOSError;
    fn try_from(other: &str) -> ActyxOSResult<Self> {
        let from = other.to_lowercase();
        let x = match from.as_ref() {
            "trace" => LogSeverity::Trace,
            "debug" => LogSeverity::Debug,
            "info" => LogSeverity::Info,
            "warn" => LogSeverity::Warn,
            "error" => LogSeverity::Error,
            _ => {
                return ax_err(
                    ActyxOSCode::ERR_INVALID_INPUT,
                    format!("\"{}\" doesn't match any known log level", from),
                )
            }
        };
        Ok(x)
    }
}
impl Default for LogSeverity {
    fn default() -> Self {
        LogSeverity::Info
    }
}
impl fmt::Display for LogSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match *self {
            LogSeverity::Trace => "TRACE",
            LogSeverity::Debug => "DEBUG",
            LogSeverity::Info => "INFO",
            LogSeverity::Warn => "WARN",
            LogSeverity::Error => "ERROR",
        };
        write!(f, "{}", name)
    }
}
impl LogSeverity {
    pub fn from_level(level: i64) -> Self {
        match level {
            0 => LogSeverity::Trace,
            1 => LogSeverity::Debug,
            2 => LogSeverity::Info,
            3 => LogSeverity::Warn,
            _ => LogSeverity::Error,
        }
    }
    pub fn to_level(self) -> i8 {
        match self {
            LogSeverity::Trace => 0,
            LogSeverity::Debug => 1,
            LogSeverity::Info => 2,
            LogSeverity::Warn => 3,
            LogSeverity::Error => 4,
        }
    }
}
