#![deny(clippy::future_not_send)]

mod actors;
mod components;
mod formats;
mod host;
mod log_tracer;
pub mod migration;
mod node;
mod node_api;
mod node_storage;
pub mod settings;
mod util;

pub use crate::{
    components::swarm_observer::{Status, SwarmObserver, SwarmState},
    node::NodeError,
    util::{init_shutdown_ceremony, shutdown_ceremony, spawn_with_name},
};
pub use formats::{node_settings, ShutdownReason};
#[cfg(not(target_os = "android"))]
pub use host::lock_working_dir;

use ::util::formats::LogSeverity;

use crate::actors::Actors;
use crate::components::swarm_observer::swarm_observer;
use crate::{
    components::{
        android::{Android, FfiMessage},
        logging::Logging,
        node_api::NodeApi,
        store::{Store, StoreRequest},
        Component, ComponentRequest,
    },
    formats::ExternalEvent,
    host::Host,
    node::NodeProcessResult,
    node::{ComponentChannel, NodeWrapper},
    settings::SettingsRequest,
    util::init_panic_hook,
};
use ::util::variable::Writer;
use ::util::SocketAddrHelper;
use acto::ActoRuntime;
use actyx_sdk::legacy::SourceId;
use anyhow::Context;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::net::ToSocketAddrs;
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr},
};
use std::{convert::TryInto, path::PathBuf, thread};
use structopt::StructOpt;
use swarm::event_store_ref::{self, EventStoreRef};

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
    _actors: Actors,
    #[allow(dead_code)]
    #[cfg(not(target_os = "android"))]
    _lock: fslock::LockFile,
}

fn bounded_channel<T>() -> (Sender<T>, Receiver<T>) {
    bounded(256)
}

fn spawn(
    working_dir: PathBuf,
    runtime: Runtime,
    bind_to: BindTo,
    log_no_color: bool,
    log_as_json: bool,
    migrate_sources_filter: Option<BTreeSet<SourceId>>,
) -> anyhow::Result<ApplicationState> {
    #[cfg(not(target_os = "android"))]
    let _lock = crate::host::lock_working_dir(&working_dir)?;
    let mut join_handles = vec![];

    let (store_tx, store_rx) = bounded_channel();
    let (node_tx, node_rx) = bounded_channel();
    let (logs_tx, logs_rx) = bounded_channel();
    let (nodeapi_tx, nodeapi_rx) = bounded_channel();

    let actors = Actors::new(node_tx.clone()).context("creating Actors")?;
    let swarm_state_writer = Writer::new(SwarmState::default());
    let swarm_state = swarm_state_writer.reader();
    let swarm_observer = actors
        .rt()
        .spawn_actor("swarm_observer", |cell| swarm_observer(cell, swarm_state_writer));
    let swarm_observer_ref = swarm_observer.me.clone();
    actors.supervise(swarm_observer.contramap(SwarmObserver::from));

    let tx = store_tx.clone();
    let event_store = EventStoreRef::new(move |e| {
        tx.try_send(ComponentRequest::Individual(StoreRequest::EventsV2(e)))
            .map_err(event_store_ref::Error::from)
    });

    let mut components = vec![
        (Store::get_type().into(), ComponentChannel::Store(store_tx.clone())),
        (NodeApi::get_type().into(), ComponentChannel::NodeApi(nodeapi_tx)),
        (Logging::get_type().into(), ComponentChannel::Logging(logs_tx)),
    ];

    // Component: Logging
    // Set up logging so tracing is set up for migration
    let logging = Logging::new(logs_rx, LogSeverity::default(), log_no_color, log_as_json);
    log::set_boxed_logger(Box::new(log_tracer::LogTracer::new([
        "yamux",
        "libp2p_gossipsub",
        "multistream_select",
        "netlink_proto",
        "libp2p_core::upgrade::apply",
    ])))
    // this may be called more than once on Android, so don’t complain
    .ok();
    log::set_max_level(log::LevelFilter::max());

    let emit_own_source = migrate_sources_filter.as_ref().map(|s| s.is_empty()).unwrap_or(false);
    migration::migrate_if_necessary(&working_dir, emit_own_source, migrate_sources_filter, false)?;

    // Host interface
    let host = Host::new(working_dir.clone()).context("creating host interface")?;
    // now set up the configured log level after initializing `Host`
    logging.set_log_level(host.get_settings().admin.log_levels.node)?;
    join_handles.push(logging.spawn().context("spawning logger")?);

    let node_id = host.get_or_create_node_id().context("getting node ID")?;
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
            node_id,
            keypair.into(),
            node.tx.clone(),
            bind_to.admin.clone(),
            nodeapi_rx,
            working_dir.join("store"),
            store_tx,
        )
    };
    join_handles.push(node_api.spawn().context("spawning node API")?);

    // Component: Store
    let store = Store::new(
        store_rx,
        event_store,
        working_dir.join("store"),
        bind_to,
        keystore,
        node_id,
        node_cycle_count,
        swarm_observer_ref.contramap(SwarmObserver::from),
    )
    .context("creating event store")?;
    join_handles.push(store.spawn().context("spawning event store")?);

    init_panic_hook(node.tx.clone());

    Ok(ApplicationState {
        join_handles,
        manager: node,
        _actors: actors,
        #[cfg(not(target_os = "android"))]
        _lock,
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
            admin: SocketAddrHelper::unspecified(4458).expect("unspecified can only fail for port 0"),
            swarm: SocketAddrHelper::unspecified(4001).expect("unspecified can only fail for port 0"),
            api: "localhost:4454".parse().unwrap(),
        }
    }
}

impl BindTo {
    /// Uses port `0` for all services. Let the OS allocate a free port.
    pub fn random() -> anyhow::Result<Self> {
        Ok(Self {
            admin: SocketAddrHelper::unspecified(0)?,
            swarm: SocketAddrHelper::unspecified(0)?,
            api: "localhost:0".parse()?,
        })
    }
}

#[derive(StructOpt, Debug)]
pub struct BindToOpts {
    #[structopt(
        long,
        parse(try_from_str = parse_port_maybe_host),
        default_value = "4458",
        long_help = "Port to bind to for management connections. Specifying a single number is \
            equivalent to “0.0.0.0:<port> [::]:<port>”, thus specifying 0 usually selects \
            different ports for IPv4 and IPv6. Specify 0.0.0.0:<port> to only use IPv4, or \
            [::]:<port> for only IPv6; you may also specify other names or addresses or leave off \
            the port number."
    )]
    /// Port to bind to for management connections.
    bind_admin: Vec<PortOrHostPort<4458>>,

    #[structopt(
        long,
        parse(try_from_str = parse_port_maybe_host),
        default_value = "4001",
        long_help = "Port to bind to for intra swarm connections. \
            The same rules apply as for the admin port."
    )]
    /// Port to bind to for intra swarm connections.
    bind_swarm: Vec<PortOrHostPort<4001>>,

    #[structopt(
        long,
        parse(try_from_str = parse_port_maybe_host),
        default_value = "localhost",
        long_help = "Port to bind to for the API used by apps. \
            The same rules apply as for the admin port, except that giving only a port binds \
            to 127.0.0.1 only. The default port is 4454."
    )]
    /// Port to bind to for the API used by apps.
    bind_api: Vec<PortOrHostPort<4454>>,
}

impl TryInto<BindTo> for BindToOpts {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<BindTo> {
        let api = fold(
            |port| SocketAddrHelper::from_ip_port(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            self.bind_api,
        )?;
        let admin = fold(SocketAddrHelper::unspecified, self.bind_admin)?;
        let swarm = fold(SocketAddrHelper::unspecified, self.bind_swarm)?;
        Ok(BindTo { admin, swarm, api })
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
enum PortOrHostPort<const DEFAULT: u16> {
    Port(u16),
    HostPort(SocketAddrHelper),
}

fn parse_port_maybe_host<const N: u16>(src: &str) -> Result<PortOrHostPort<N>, String> {
    let port = match src.parse::<u16>() {
        Ok(p) => return Ok(PortOrHostPort::Port(p)),
        Err(e) => e,
    };
    let host_string = match SocketAddrHelper::from_host_string(src) {
        Ok(p) => return Ok(PortOrHostPort::HostPort(p)),
        Err(e) => e,
    };
    let multiaddr = match SocketAddrHelper::parse_multiaddr(src) {
        Ok(m) => return Ok(PortOrHostPort::HostPort(m)),
        Err(e) => e,
    };
    let sock_addr = match (src, N).to_socket_addrs() {
        Ok(i) => return Ok(PortOrHostPort::HostPort(i.collect())),
        Err(e) => e,
    };
    Err(format!(
        "cannot interpret `{}`:\n  as port number: {:#}\n  as <host:port>: {:#}\
        \n  as multiaddr:   {:#}\n  as IP or name:  {:#}",
        src, port, host_string, multiaddr, sock_addr
    ))
}

fn fold<const N: u16>(
    port: impl FnOnce(u16) -> anyhow::Result<SocketAddrHelper>,
    input: Vec<PortOrHostPort<N>>,
) -> anyhow::Result<SocketAddrHelper> {
    if input.is_empty() {
        anyhow::bail!("no value provided");
    }
    let mut found_port = None;
    let mut host_port: Option<SocketAddrHelper> = None;
    for i in input.into_iter() {
        match i {
            PortOrHostPort::Port(p) => {
                if found_port.is_some() {
                    anyhow::bail!("Multiple single port directives not supported");
                } else if host_port.is_some() {
                    anyhow::bail!("Both port directive and host:port combination not supported");
                } else {
                    found_port.replace(p);
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
    found_port
        .map(port)
        .or_else(|| host_port.map(Ok))
        .expect("Input must not be empty")
}

impl ApplicationState {
    /// Bootstraps the application, and returns a handle structure.
    pub fn spawn(
        base_dir: PathBuf,
        runtime: Runtime,
        bind_to: BindTo,
        log_no_color: bool,
        log_as_json: bool,
        migrate_sources_filter: Option<BTreeSet<SourceId>>,
    ) -> anyhow::Result<Self> {
        spawn(
            base_dir,
            runtime,
            bind_to,
            log_no_color,
            log_as_json,
            migrate_sources_filter,
        )
        .context("spawning core infrastructure")
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
        self.shutdown(ShutdownReason::TriggeredByHost);
    }
}
