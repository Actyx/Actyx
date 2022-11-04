use super::store::StoreTx;
use crate::{
    components::{Component, ComponentRequest},
    formats::ExternalEvent,
    node_settings::Settings,
};
use actyx_sdk::NodeId;
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use libp2p::PeerId;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use util::SocketAddrHelper;

impl NodeApi {
    pub(crate) fn new(
        node_id: NodeId,
        keypair: libp2p::core::identity::Keypair,
        sender: Sender<ExternalEvent>,
        bind_to: SocketAddrHelper,
        rx: Receiver<ComponentRequest<()>>,
        store_dir: PathBuf,
        store: StoreTx,
    ) -> Self {
        Self {
            node_id,
            rx,
            keypair,
            bind_to,
            sender,
            rt: None,
            settings: Default::default(),
            store_dir,
            store,
        }
    }
}

pub struct NodeApi {
    node_id: NodeId,
    rx: Receiver<ComponentRequest<()>>,
    keypair: libp2p::core::identity::Keypair,
    bind_to: SocketAddrHelper,
    sender: Sender<ExternalEvent>,
    rt: Option<tokio::runtime::Runtime>,
    settings: Arc<Mutex<NodeApiSettings>>,
    store_dir: PathBuf,
    store: StoreTx,
}
#[derive(Default, PartialEq, Eq, Clone)]
pub struct NodeApiSettings {
    pub authorized_keys: Vec<PeerId>,
}
impl Component<(), NodeApiSettings> for NodeApi {
    fn get_type() -> &'static str {
        "Admin"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<()>> {
        &self.rx
    }
    fn extract_settings(&self, s: Settings) -> Result<NodeApiSettings> {
        let authorized_keys = s.admin.authorized_users.iter().cloned().map(Into::into).collect();
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
    fn start(&mut self, snd: Sender<anyhow::Result<()>>) -> Result<()> {
        debug_assert!(self.rt.is_none());
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        rt.block_on(crate::node_api::mk_swarm(
            self.node_id,
            self.keypair.clone(),
            self.sender.clone(),
            self.bind_to.clone(),
            self.store_dir.clone(),
            self.store.clone(),
            self.settings.clone(),
        ))?;

        // mk_swarm has bound the listen sockets, so declare victory
        snd.send(Ok(()))?;

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
