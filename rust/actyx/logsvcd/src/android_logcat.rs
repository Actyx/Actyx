use crate::{
    error::{LogsvcdError, Result},
    storage::StorageWrapper,
};
use android_log_sys::LogPriority;
use crossbeam::channel::Receiver;
use rogcat::parser::Parser;
use std::ffi::CString;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
use util::formats::{LogEvent, LogRequest, LogSeverity};
static ANDROID_PACKAGE_PREFIX: &str = "com.actyx.android";
static ANDROID_NODE_LOG_PREFIX: &str = "com.actyx.node";
static ANDROID_NODE_LOG_PREFIX_SHORT: &str = "c.a.n";

/// Polls logcat according to the supplied `filter` and inserts new log into the DB
/// This is a blocking operation.
pub fn logcat_loop(storage: StorageWrapper, filter: String) -> Result<()> {
    let mut child = Command::new("logcat")
        // -T 1: Start from now on and follow
        .arg("-T")
        .arg("1")
        .arg(filter)
        .stdout(Stdio::piped())
        .spawn()?;

    let reader = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| LogsvcdError::GenericError("Unable to acquire stdout".to_string()))?,
    );

    let mut parser = Parser::default();

    for line in reader.lines() {
        let line = line?;
        let record = parser.parse(&line);
        // avoid loops ..
        if !(record.tag.starts_with(ANDROID_NODE_LOG_PREFIX) || record.tag.starts_with(ANDROID_NODE_LOG_PREFIX_SHORT)) {
            let log: LogRequest = record.into();
            storage
                .spawn_mut(|s| s.add_logs(vec![log]))
                .map_err(|e| format!("Unable to spawn into DB thread {}", e))??;
        }
    }

    Ok(())
}

fn android_log(record: AndroidLog) {
    let priority = match record.level {
        LogSeverity::Trace => LogPriority::VERBOSE,
        LogSeverity::Debug => LogPriority::DEBUG,
        LogSeverity::Info => LogPriority::INFO,
        LogSeverity::Warn => LogPriority::WARN,
        LogSeverity::Error => LogPriority::ERROR,
        LogSeverity::Fatal => LogPriority::FATAL,
    } as i32;
    let tag = CString::new(record.tag).unwrap().into_raw();
    let msg = CString::new(record.message).unwrap().into_raw();
    unsafe {
        android_log_sys::__android_log_write(priority, tag, msg);
    }
}
pub struct AndroidLog {
    level: LogSeverity,
    tag: String,
    message: String,
}

impl From<LogEvent> for AndroidLog {
    fn from(e: LogEvent) -> Self {
        let level = e.severity.into();
        let tag = format!("{}.{}", ANDROID_NODE_LOG_PREFIX, e.log_name);
        let message = e.message;
        Self { level, tag, message }
    }
}

pub fn logs_to_logcat(rx: Receiver<Vec<LogEvent>>) {
    loop {
        if let Ok(logs) = rx.recv() {
            for val in logs {
                if !val.producer_name.starts_with(ANDROID_PACKAGE_PREFIX) {
                    android_log(val.into());
                }
            }
        }
    }
}
