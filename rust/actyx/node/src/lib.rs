#![deny(clippy::future_not_send)]

mod components;
mod formats;
mod host;
mod node;
mod node_api;
mod node_storage;
pub mod settings;
mod util;

pub use crate::node::NodeError;
pub use crate::util::spawn_with_name;
#[cfg(not(windows))]
pub use crate::util::unix_shutdown::shutdown_ceremony;
pub use formats::{node_settings, ShutdownReason};

use crate::{
    components::{
        android::{Android, FfiMessage},
        logging::Logging,
        node_api::NodeApi,
        store::Store,
        Component,
    },
    formats::ExternalEvent,
    host::Host,
    node::NodeProcessResult,
    node::{ComponentChannel, NodeWrapper},
    settings::SettingsRequest,
    util::init_panic_hook,
};
use ::util::SocketAddrHelper;
use anyhow::Context;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::{convert::TryInto, path::PathBuf, str::FromStr, thread};
use structopt::StructOpt;

// Rust defaults to use the system allocator, which seemed to be the fastest
// allocator generally available for our use case [0]. For production, the Actyx
// binaries are compiled statically using the musl toolchain. The allocator
// shipped with musl 0.9.9 performs worse than the system allocator. This is why
// for musl targets, this falls back to jemalloc.
// Once musl updates its allocator, this should be re-evaluated.
// Jemalloc doesn't support i686, so this is only done on 64 bit target archs.
// [0]: https://github.com/Actyx/Cosmos/issues/4198
#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub enum Runtime {
    Android { ffi_sink: Sender<FfiMessage> },
    Windows,
    Linux,
}

pub struct ApplicationState {
    pub join_handles: Vec<thread::JoinHandle<()>>,
    pub manager: NodeWrapper,
}

fn bounded_channel<T>() -> (Sender<T>, Receiver<T>) {
    bounded(256)
}

fn spawn(working_dir: PathBuf, runtime: Runtime, bind_to: BindTo) -> anyhow::Result<ApplicationState> {
    let mut join_handles = vec![];

    let (store_tx, store_rx) = bounded_channel();
    let (node_tx, node_rx) = bounded_channel();
    let (logs_tx, logs_rx) = bounded_channel();
    let (nodeapi_tx, nodeapi_rx) = bounded_channel();

    let mut components = vec![
        (Store::get_type().into(), ComponentChannel::Store(store_tx.clone())),
        (NodeApi::get_type().into(), ComponentChannel::NodeApi(nodeapi_tx)),
        (Logging::get_type().into(), ComponentChannel::Logging(logs_tx)),
    ];

    // Host interface
    let host = Host::new(working_dir.clone()).context("creating host interface")?;
    let node_id = host.get_or_create_node_id().context("getting node ID")?;

    // Component: Logging
    let logging = Logging::new(node_id, logs_rx, host.working_dir());
    join_handles.push(logging.spawn().context("spawning logger")?);
    tracing::debug!("NodeID: {}", node_id);

    // Runtime specifics
    match runtime {
        Runtime::Android { ffi_sink } => {
            let (runtime_tx, runtime_rx) = bounded_channel();
            components.push((Android::get_type().into(), ComponentChannel::Android(runtime_tx)));
            let android = Android::new(node_tx.clone(), runtime_rx, ffi_sink);
            join_handles.push(android.spawn()?);
        }
        Runtime::Linux | Runtime::Windows => {}
    };

    let keystore = host.get_keystore();

    let db = host.get_db_handle();
    let node_cycle_count = host.get_cycle_count().context("getting cycle count")?;
    // THE node :-)
    let node = NodeWrapper::new((node_tx, node_rx), components, host).context("creating node core")?;

    // Component: NodeApi
    let node_api = {
        let keypair = keystore
            .read()
            .get_pair(node_id.into())
            // Should have been created by the call to
            // `CryptoCell::get_or_create_node_id` within this bootstrap routine
            // earlier
            .context("No keypair registered for node")?;
        NodeApi::new(
            keypair.into(),
            node.tx.clone(),
            bind_to.admin.clone(),
            nodeapi_rx,
            store_tx,
        )
    };
    join_handles.push(node_api.spawn().context("spawning node API")?);

    // Component: Store
    let store = Store::new(
        store_rx,
        working_dir.join("store"),
        bind_to,
        keystore,
        node_id,
        db,
        node_cycle_count,
    )
    .context("creating event store")?;
    join_handles.push(store.spawn().context("spawning event store")?);

    init_panic_hook(node.tx.clone());

    Ok(ApplicationState {
        join_handles,
        manager: node,
    })
}
pub type NodeLifecycleResult = Receiver<NodeProcessResult<()>>;

#[derive(Debug, Clone)]
pub struct BindTo {
    pub admin: SocketAddrHelper,
    pub swarm: SocketAddrHelper,
    pub api: SocketAddrHelper,
}

impl Default for BindTo {
    fn default() -> Self {
        Self {
            admin: SocketAddrHelper::unspecified(4458),
            swarm: SocketAddrHelper::unspecified(4001),
            api: "localhost:4454".parse().unwrap(),
        }
    }
}

impl BindTo {
    /// Uses port `0` for all services. Let the OS allocate a free port.
    pub fn random() -> Self {
        Self {
            admin: SocketAddrHelper::unspecified(0),
            swarm: SocketAddrHelper::unspecified(0),
            api: "localhost:0".parse().unwrap(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct BindToOpts {
    /// Port to bind to for the management API.
    #[structopt(long, parse(try_from_str = parse_port_maybe_host), default_value = "4458")]
    bind_admin: Vec<PortOrHostPort>,

    /// Port to bind to for intra swarm connections.
    #[structopt(long, parse(try_from_str = parse_port_maybe_host), default_value = "4001")]
    bind_swarm: Vec<PortOrHostPort>,

    /// Port bind to for the API.
    #[structopt(long, parse(try_from_str = parse_port_maybe_host), default_value = "4454")]
    bind_api: Vec<PortOrHostPort>,
}
impl TryInto<BindTo> for BindToOpts {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<BindTo> {
        let mut bind_to = BindTo::default();
        PortOrHostPort::maybe_set(&mut bind_to.api, self.bind_api)?;
        PortOrHostPort::maybe_set(&mut bind_to.admin, self.bind_admin)?;
        PortOrHostPort::maybe_set(&mut bind_to.swarm, self.bind_swarm)?;
        Ok(bind_to)
    }
}

// This supports plain ports, host:port combinations and multiaddrs (although
// only the subset which `SocketAddrHelper` supports), host:port combination is
// an undocumented feature. Users should be nudged to build local first apps,
// thus APIs needed during app runtime (like the Event Service) should only bind
// to localhost. An escape hatch is needed for certain situations, like
// containerization though. Changing the default ports however might be
// necessary more frequently, and this is why that is offered here primarily.
#[derive(Debug)]
enum PortOrHostPort {
    Port(u16),
    HostPort(SocketAddrHelper),
}

fn parse_port_maybe_host(src: &str) -> anyhow::Result<PortOrHostPort> {
    if let Ok(port) = src.parse::<u16>() {
        Ok(PortOrHostPort::Port(port))
    } else {
        SocketAddrHelper::from_str(src).map(PortOrHostPort::HostPort)
    }
}

fn fold(input: Vec<PortOrHostPort>) -> anyhow::Result<Option<PortOrHostPort>> {
    if input.is_empty() {
        return Ok(None);
    }
    let mut found_port = None;
    let mut host_port: Option<SocketAddrHelper> = None;
    for i in input.into_iter() {
        match i {
            x @ PortOrHostPort::Port(_) => {
                if found_port.is_some() {
                    anyhow::bail!("Multiple single port directives not supported");
                } else if host_port.is_some() {
                    anyhow::bail!("Both port directive and host:port combination not supported");
                } else {
                    found_port.replace(x);
                }
            }
            PortOrHostPort::HostPort(addr) => {
                if found_port.is_some() {
                    anyhow::bail!("Both port directive and host:port combination not supported");
                } else if let Some(x) = host_port.as_mut() {
                    x.append(addr);
                } else {
                    let _ = host_port.replace(addr);
                }
            }
        }
    }
    let ret = found_port
        .or_else(|| host_port.map(PortOrHostPort::HostPort))
        .expect("Input must not be empty");
    Ok(Some(ret))
}

impl PortOrHostPort {
    pub fn maybe_set(target: &mut SocketAddrHelper, input: Vec<PortOrHostPort>) -> anyhow::Result<()> {
        if let Some(port_or_host) = fold(input)? {
            match port_or_host {
                PortOrHostPort::Port(port) => target.set_port(port),
                PortOrHostPort::HostPort(addr) => {
                    let _ = std::mem::replace(target, addr);
                }
            }
        }
        Ok(())
    }
}

impl ApplicationState {
    /// Bootstraps the application, and returns a handle structure.
    pub fn spawn(base_dir: PathBuf, runtime: Runtime, bind_to: BindTo) -> anyhow::Result<Self> {
        spawn(base_dir, runtime, bind_to).context("spawning core infrastructure")
    }

    pub fn handle_settings_request(&self, message: SettingsRequest) {
        self.manager.tx.send(ExternalEvent::SettingsRequest(message)).unwrap()
    }

    pub fn shutdown(&mut self, reason: ShutdownReason) {
        let _ = self.manager.tx.send(ExternalEvent::ShutdownRequested(reason));
        for h in self.join_handles.drain(..) {
            tracing::debug!(
                "Waiting for thread (ID: \"{:?}\", Name: \"{}\") to join",
                h.thread().id(),
                h.thread().name().unwrap_or("Unknown")
            );
            h.join().unwrap();
        }
    }
}

impl Drop for ApplicationState {
    fn drop(&mut self) {
        self.shutdown(ShutdownReason::TriggeredByHost)
    }
}
