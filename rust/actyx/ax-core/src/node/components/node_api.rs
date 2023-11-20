use super::store::StoreTx;
use crate::{
    node::{
        components::{Component, ComponentRequest},
        formats::ExternalEvent,
        node_settings::Settings,
    },
    util::SocketAddrHelper,
};
use anyhow::Result;
use ax_sdk::NodeId;
use crossbeam::channel::{Receiver, Sender};
use libp2p::PeerId;
use parking_lot::Mutex;
use std::{
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

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
        extract_settings_into_node_settings(s)
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
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("node-api-worker-{}", id)
            })
            .worker_threads(2)
            .enable_all()
            .build()?;

        rt.block_on(crate::node::node_api::mk_swarm(
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

fn extract_settings_into_node_settings(s: Settings) -> Result<NodeApiSettings> {
    let authorized_keys: Vec<PeerId> = s
        .admin
        .authorized_users
        .iter()
        .enumerate()
        .filter_map(|(i, pk)| match crate::crypto::PublicKey::from_str(pk) {
            Ok(pk) => Some(PeerId::from(pk)),
            Err(_) => {
                tracing::warn!("Found invalid entry in config/admin/authorizedUsers at index: {}", i);
                None
            }
        })
        .collect();
    Ok(NodeApiSettings { authorized_keys })
}

#[cfg(test)]
mod tests {
    use crate::node::{components::node_api::extract_settings_into_node_settings, node_settings::Settings};

    #[test]
    pub fn sample_with_invalid_authorized_users() {
        let mut sample_json = serde_json::to_value(Settings::sample()).unwrap();
        if let serde_json::Value::Object(sample) = &mut sample_json {
            let admin_json = sample.get_mut("admin").expect("Settings::sample().admin undefined");
            let authorized_users = admin_json
                .get_mut("authorizedUsers")
                .expect("Settings::sample().admin.authorized_users is undefined");
            if let serde_json::Value::Array(authorized_users_as_array) = authorized_users {
                // valid
                authorized_users_as_array.push("0BvjSPuvSFnxeJu+PWfFtZBpnfcrjh6pcz1e6kQjxNhg=".into());
                authorized_users_as_array.push("0OAapA3dk0KzFVJrEEYwvP3CLKY/UEYImE+B8oV+19EU=".into());
                // invalid
                authorized_users_as_array.push("0FtjBTIiGoM3LlS4xJcFnUxkPItCBWWlOmNnJgmTtTLQ=".into());
            } else {
                panic!("Settings::sample().admin.authorizedUsers is not an array");
            }
        } else {
            panic!("Settings::sample() is not an object");
        }

        let settings = serde_json::from_str::<Settings>(sample_json.to_string().as_str()).unwrap();
        let node_api_settings = extract_settings_into_node_settings(settings).unwrap();
        assert_eq!(node_api_settings.authorized_keys.len(), 2);
    }
}
