use super::{logging::LoggingTx, store::StoreTx};
use crate::{
    components::{Component, ComponentRequest},
    formats::ExternalEvent,
    os_settings::Settings,
};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use libp2p::PeerId;
use parking_lot::Mutex;
use std::sync::Arc;
use util::SocketAddrHelper;

impl NodeApi {
    pub(crate) fn new(
        keypair: libp2p::core::identity::Keypair,
        sender: Sender<ExternalEvent>,
        bind_to: SocketAddrHelper,
        rx: Receiver<ComponentRequest<()>>,
        logsvcd: LoggingTx,
        store: StoreTx,
    ) -> Self {
        Self {
            rx,
            keypair,
            bind_to,
            sender,
            logsvcd,
            rt: None,
            settings: Default::default(),
            store,
        }
    }
}

pub struct NodeApi {
    rx: Receiver<ComponentRequest<()>>,
    keypair: libp2p::core::identity::Keypair,
    bind_to: SocketAddrHelper,
    sender: Sender<ExternalEvent>,
    logsvcd: LoggingTx,
    rt: Option<tokio::runtime::Runtime>,
    settings: Arc<Mutex<NodeApiSettings>>,
    store: StoreTx,
}
#[derive(Default, PartialEq, Clone)]
pub struct NodeApiSettings {
    pub authorized_keys: Vec<PeerId>,
}
impl Component<(), NodeApiSettings> for NodeApi {
    fn get_type(&self) -> &'static str {
        "Admin"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<()>> {
        &self.rx
    }
    fn extract_settings(&self, s: Settings) -> Result<NodeApiSettings> {
        let authorized_keys = s.general.authorized_keys.iter().cloned().map(Into::into).collect();
        Ok(NodeApiSettings { authorized_keys })
    }
    fn handle_request(&mut self, _: ()) -> Result<()> {
        Ok(())
    }
    fn set_up(&mut self, s: NodeApiSettings) -> bool {
        let mut g = self.settings.lock();
        *g = s;
        false
    }
    fn start(&mut self, _: Sender<anyhow::Error>) -> Result<()> {
        debug_assert!(self.rt.is_none());
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let (_, swarm) = rt.block_on(crate::node_api::mk_swarm(
            self.keypair.clone(),
            self.sender.clone(),
            self.bind_to.clone(),
            self.logsvcd.clone(),
            self.store.clone(),
            self.settings.clone(),
        ))?;
        let _ = rt.spawn(crate::node_api::start(swarm));
        self.rt = Some(rt);
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(rt) = self.rt.take() {
            drop(rt)
        }
        Ok(())
    }
}
