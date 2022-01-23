use self::logging_sink::LoggingSink;
use super::{Component, ComponentRequest};
use crate::node_settings::Settings;
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::sync::Arc;
use util::formats::LogSeverity;

mod logging_sink;
pub struct Logging {
    rx: Receiver<ComponentRequest<()>>,
    logging_sink: Arc<Mutex<LoggingSink>>,
}

impl Component<(), LogSeverity> for Logging {
    fn get_type() -> &'static str {
        "logging"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<()>> {
        &self.rx
    }
    fn handle_request(&mut self, _: ()) -> Result<()> {
        Ok(())
    }
    fn extract_settings(&self, settings: Settings) -> Result<LogSeverity> {
        Ok(settings.admin.log_levels.node)
    }
    fn set_up(&mut self, settings: LogSeverity) -> bool {
        if let Err(e) = self.logging_sink.lock().set_level(settings) {
            eprintln!("Error setting new log level: {}", e);
        }
        false
    }
    fn start(&mut self, snd: Sender<anyhow::Result<()>>) -> Result<()> {
        snd.send(Ok(()))?;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}
impl Logging {
    pub fn new(
        rx: Receiver<ComponentRequest<()>>,
        level: LogSeverity,
        log_no_color: bool,
        log_as_json: Option<bool>,
    ) -> Self {
        let logging_sink = Arc::new(Mutex::new(LoggingSink::new(level, log_no_color, log_as_json)));
        Self { rx, logging_sink }
    }
    pub fn set_log_level(&self, level: LogSeverity) -> anyhow::Result<()> {
        self.logging_sink.lock().set_level(level)?;
        Ok(())
    }
}
