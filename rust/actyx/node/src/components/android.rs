use super::Component;
use crate::{
    components::ComponentRequest,
    formats::{ExternalEvent, ShutdownReason},
    os_settings::Settings,
};
use anyhow::Result;
use crossbeam::channel::{self, Receiver, Sender};
use ffi_support::rust_string_to_c;
use std::{convert::TryFrom, os::raw::c_char};

#[derive(Debug)]
pub struct FfiMessage {
    code: i32,
    message: String,
}
mod ffi_codes {
    // Internal error happened
    pub const NODE_STOPPED_BY_NODE: i32 = 10;
    // Wired in via axnode_shutdown from android when app stopped by the user
    pub const NODE_STOPPED_BY_NODE_UI: i32 = 11;
    // Wired in via axnode_shutdown from android when app stopped/killed by android
    pub const NODE_STOPPED_BY_HOST: i32 = 12;
}

impl TryFrom<i32> for ShutdownReason {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        use ffi_codes::*;
        match value {
            NODE_STOPPED_BY_NODE_UI => Ok(ShutdownReason::TriggeredByUser),
            NODE_STOPPED_BY_HOST => Ok(ShutdownReason::TriggeredByHost),
            x => anyhow::bail!(format!("Unsupported error code! {}", x)),
        }
    }
}

impl Into<FfiMessage> for ShutdownReason {
    fn into(self) -> FfiMessage {
        let (code, message) = match self {
            ShutdownReason::TriggeredByUser => (ffi_codes::NODE_STOPPED_BY_NODE_UI, "".to_string()),

            ShutdownReason::TriggeredByHost => (ffi_codes::NODE_STOPPED_BY_HOST, "".to_string()),

            ShutdownReason::Internal(err) => (ffi_codes::NODE_STOPPED_BY_NODE, err.to_string()),
        };
        FfiMessage::new(code, message)
    }
}
impl FfiMessage {
    pub fn new(code: i32, message: String) -> Self {
        Self { code, message }
    }
}
impl Into<(i32, *mut c_char)> for FfiMessage {
    // This will leak memory, so consumers need to make sure to eventually free
    // it again.
    fn into(self) -> (i32, *mut c_char) {
        (self.code, rust_string_to_c(self.message))
    }
}
#[allow(dead_code)]
pub(crate) struct Android {
    rx: Receiver<ComponentRequest<()>>,
    android_tx: Sender<FfiMessage>,
    sender: Sender<ExternalEvent>,
}

impl Android {
    pub fn new(
        sender: Sender<ExternalEvent>,
        rx: Receiver<ComponentRequest<()>>,
        android_tx: Sender<FfiMessage>,
    ) -> Self {
        Self { android_tx, rx, sender }
    }
}

impl Component<(), ()> for Android {
    fn get_rx(&self) -> &channel::Receiver<ComponentRequest<()>> {
        &self.rx
    }
    fn get_type(&self) -> &'static str {
        "android"
    }
    fn handle_request(&mut self, _: ()) -> Result<()> {
        Ok(())
    }
    fn extract_settings(&self, _: Settings) -> Result<()> {
        Ok(())
    }
    fn start(&mut self, _: Sender<anyhow::Error>) -> Result<()> {
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn loop_on_rx(self) -> Result<()> {
        while let Ok(msg) = self.rx.recv() {
            if let ComponentRequest::Shutdown(r) = msg {
                self.android_tx.send(r.into())?;
                break;
            }
        }
        Ok(())
    }
}
