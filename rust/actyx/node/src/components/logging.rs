use super::{Component, ComponentRequest};
use crate::os_settings::Settings;
use actyxos_sdk::NodeId;
use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};
use logsvcd::{DynamicConfig, GetLogRequest, LogConfig, LogServiceWrapper, LoggingSink};
use parking_lot::Mutex;
use std::{path::Path, sync::Arc};
use util::formats::{LogRequest, LogSeverity};

pub type LoggingTx = Sender<ComponentRequest<LoggingRequest>>;
pub struct Logging {
    log: LogServiceWrapper,
    rx: Receiver<ComponentRequest<LoggingRequest>>,
    tx_logsvcd_cfg: Sender<DynamicConfig>,
    node_id: NodeId,
    logging_sink: Arc<Mutex<LoggingSink>>,
}

#[allow(dead_code)]
pub enum LoggingRequest {
    GetLogRequest(GetLogRequest),
    // TODO: to be used by console http api
    PublishLog(LogRequest),
}

#[derive(Clone, PartialEq)]
pub struct LoggingConfig {
    dynamic: DynamicConfig,
    log_level: LogSeverity,
}

impl Component<LoggingRequest, LoggingConfig> for Logging {
    fn get_type(&self) -> &'static str {
        "logging"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<LoggingRequest>> {
        &self.rx
    }
    fn handle_request(&mut self, r: LoggingRequest) -> Result<()> {
        match r {
            LoggingRequest::GetLogRequest(g) => self.log.tx.send(g)?,
            LoggingRequest::PublishLog(p) => self.log.publish_tx.send(p)?,
        }
        Ok(())
    }
    fn extract_settings(&self, settings: Settings) -> Result<LoggingConfig> {
        let node_name = settings.admin.display_name;

        let dynamic = logsvcd::DynamicConfig {
            node_id: self.node_id,
            node_name,
        };
        let cfg = LoggingConfig {
            dynamic,
            log_level: settings.admin.log_levels.node,
        };
        Ok(cfg)
    }
    fn set_up(&mut self, settings: LoggingConfig) -> bool {
        if let Err(e) = self.logging_sink.lock().set_level(settings.log_level) {
            eprintln!("Error setting new log level: {}", e);
        }
        self.tx_logsvcd_cfg.send(settings.dynamic).unwrap();
        // logsvcd is never restarted; new configs are conveyed through a
        // channel above
        false
    }
    fn start(&mut self, _: Sender<anyhow::Error>) -> Result<()> {
        // Started immediately in `Logging::new`
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        // TODO: Shutdown
        Ok(())
    }
}
impl Logging {
    pub fn new(node_id: NodeId, rx: Receiver<ComponentRequest<LoggingRequest>>, working_dir: impl AsRef<Path>) -> Self {
        let (tx_logsvcd_cfg, log_cfg_rx) = bounded(8);

        let log_config = LogConfig::with_dir(working_dir, log_cfg_rx);
        let log = LogServiceWrapper::new(log_config, node_id);
        let logging_sink = Arc::new(Mutex::new(LoggingSink::new(
            LogSeverity::default(),
            log.publish_tx.clone(),
        )));
        Self {
            node_id,
            log,
            rx,
            tx_logsvcd_cfg,
            logging_sink,
        }
    }
}
