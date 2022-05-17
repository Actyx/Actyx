#![cfg(target_os = "linux")]

pub mod api;

use actyx_sdk::NodeId;
use anyhow::{bail, Result};
use async_std::{future, task};
use futures::{
    future::{select, Either},
    prelude::*,
};
use netsim_embed::{DelayBuffer, Ipv4Range, Machine, Netsim};
use std::{
    borrow::Borrow,
    collections::BTreeSet,
    fmt::Display,
    net::SocketAddr,
    str::FromStr,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use swarm_cli::{multiaddr, Command, Config, EphemeralEventsConfigWrapper, Event, Multiaddr, PeerId};
use tempdir::TempDir;

pub mod util;

#[derive(StructOpt)]
pub struct HarnessOpts {
    #[structopt(long, default_value = "2")]
    pub n_nodes: usize,

    #[structopt(long, default_value = "0")]
    pub delay_ms: u64,

    #[structopt(long, default_value = "0")]
    pub n_bootstrap: usize,

    #[structopt(long)]
    pub enable_mdns: bool,

    #[structopt(long)]
    pub enable_fast_path: bool,

    #[structopt(long)]
    pub enable_slow_path: bool,

    #[structopt(long)]
    pub enable_root_map: bool,

    #[structopt(long)]
    pub enable_discovery: bool,

    #[structopt(long)]
    pub enable_metrics: bool,

    #[structopt(long)]
    pub enable_api: Option<SocketAddr>,

    #[structopt(long)]
    pub ephemeral_events: Option<EphemeralEventsConfigWrapper>,

    #[structopt(long)]
    pub max_leaf_count: Option<usize>,
}

pub trait MachineExt {
    fn peer_id(&self) -> PeerId;
    fn node_id(&self) -> NodeId;
    fn multiaddr(&self) -> Multiaddr;
}

impl<C, E> MachineExt for netsim_embed::Machine<C, E> {
    fn peer_id(&self) -> PeerId {
        swarm_cli::keypair(self.id().0 as u64).into()
    }

    fn node_id(&self) -> NodeId {
        swarm_cli::keypair(self.id().0 as u64).into()
    }

    fn multiaddr(&self) -> Multiaddr {
        format!("/ip4/{}/tcp/30000", self.addr()).parse().unwrap()
    }
}

pub trait MultiaddrExt {
    fn is_loopback(&self) -> bool;
}

impl MultiaddrExt for Multiaddr {
    fn is_loopback(&self) -> bool {
        if let Some(multiaddr::Protocol::Ip4(addr)) = self.iter().next() {
            if !addr.is_loopback() {
                return false;
            }
        }
        true
    }
}

pub fn setup_env() -> Result<()> {
    ::util::setup_logger();
    netsim_embed::unshare_user()?;
    Ok(())
}

pub fn run_netsim<F, F2, E>(opts: HarnessOpts, f: F) -> Result<()>
where
    F: FnOnce(Netsim<Command, E>) -> F2,
    F2: Future<Output = Result<()>> + Send,
    E: FromStr<Err = anyhow::Error> + Display + Send + 'static,
{
    let temp_dir = TempDir::new("swarm-harness")?;
    async_global_executor::block_on(async move {
        let mut sim = Netsim::new();
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());
        tracing::warn!("using network {:?}", sim.network(net).range());
        let mut addrs = Vec::with_capacity(opts.n_bootstrap);
        let mut bootstrap = Vec::with_capacity(opts.n_bootstrap);
        for i in 0..opts.n_bootstrap {
            let peer_id: PeerId = swarm_cli::keypair(i as u64).into();
            let addr = sim.network_mut(net).unique_addr();
            let maddr = format!("/ip4/{}/tcp/30000/p2p/{}", addr, peer_id);
            addrs.push(addr);
            bootstrap.push(maddr.parse().unwrap());
        }
        for i in 0..opts.n_nodes {
            let cfg = Config {
                path: Some(temp_dir.path().join(i.to_string())),
                node_name: None,
                keypair: i as _,
                listen_on: vec!["/ip4/0.0.0.0/tcp/30000".parse().unwrap()],
                bootstrap: bootstrap.clone(),
                external: vec![],
                enable_mdns: opts.enable_mdns,
                enable_fast_path: opts.enable_fast_path,
                enable_slow_path: opts.enable_slow_path,
                enable_root_map: opts.enable_root_map,
                enable_discovery: opts.enable_discovery,
                enable_metrics: opts.enable_metrics,
                enable_api: opts.enable_api,
                ephemeral_events: opts.ephemeral_events.clone(),
                max_leaf_count: opts.max_leaf_count,
            };
            let mut delay = DelayBuffer::new();
            delay.set_delay(Duration::from_millis(opts.delay_ms));
            let cmd = async_process::Command::from(cfg);
            let machine = sim.spawn_machine(cmd, Some(delay)).await;
            sim.plug(machine, net, addrs.get(i).copied()).await;
            let m = sim.machine(machine);
            tracing::warn!(
                "{} started with address {} and peer id {}",
                machine,
                m.addr(),
                m.peer_id()
            );
        }
        f(sim).await
    })
}

pub struct WaitResult<T> {
    value: Option<T>,
}

impl<T> WaitResult<T> {
    pub fn value(self) -> Option<T> {
        self.value
    }
}

impl From<bool> for WaitResult<()> {
    fn from(b: bool) -> Self {
        if b {
            Self { value: Some(()) }
        } else {
            Self { value: None }
        }
    }
}

impl<T> From<Option<T>> for WaitResult<T> {
    fn from(value: Option<T>) -> Self {
        Self { value }
    }
}

type Selector<'a, T> = dyn Fn(&Event) -> WaitResult<T> + Send + Sync + 'a;

pub fn selector<'a, T, F, R>(f: F) -> Box<Selector<'a, T>>
where
    F: Fn(&Event) -> R + Send + Sync + 'a,
    R: Into<WaitResult<T>>,
{
    Box::new(move |ev| f(ev).into())
}

/// Like `matches!()` but allows you to extract a result from the matched pattern.
/// Also supports `if` guard after the pattern and before the `=>`.
/// The result is wrapped in an option, which is `None` if the pattern & guard didn’t match.
///
/// ```
/// use swarm_harness::m;
///
/// let x: Result<&str, ()> = Ok("hello");
/// let s: Option<&str> = m!(x, Ok(s) => s);
/// ```
#[macro_export]
macro_rules! m {
    ($v:expr, $p:pat => $e:expr) => {
        match $v {
            $p => Some($e),
            _ => None,
        }
    };
    ($v:expr, $p:pat if $c:expr => $e:expr) => {
        match $v {
            $p if $c => Some($e),
            _ => None,
        }
    };
}

pub async fn select_single<'a, F, T, R>(machine: &mut Machine<Command, Event>, timeout: Duration, f: F) -> T
where
    F: Fn(&Event) -> R + Send + Sync + 'a,
    R: Into<WaitResult<T>>,
{
    future::timeout(timeout, select_multi_internal(machine, vec![selector(f)]))
        .await
        .unwrap()
        .remove(0)
}

/// run multiple selections where you don’t know the order in advance (or don’t care)
///
/// The individual things to check are most conveniently constructed using the `selector()` function.
pub async fn select_multi<T>(
    machine: &mut Machine<Command, Event>,
    timeout: Duration,
    things: Vec<Box<Selector<'_, T>>>,
) -> Vec<T> {
    future::timeout(timeout, select_multi_internal(machine, things))
        .await
        .unwrap()
}

async fn select_multi_internal<T>(machine: &mut Machine<Command, Event>, things: Vec<Box<Selector<'_, T>>>) -> Vec<T> {
    let mut items = things.len();
    let mut things = things.into_iter().map(Some).collect::<Vec<_>>();
    let mut res = Vec::new();
    res.resize_with(items, || None);
    let id = machine.id();
    while items > 0 {
        let timer = Instant::now();
        machine
            .select(|ev| {
                for (idx, t) in things.iter_mut().enumerate() {
                    if let Some(f) = t {
                        if let Some(r) = f(ev).value() {
                            tracing::info!("{} saw {:?} after {:.1}sec", id, ev, timer.elapsed().as_secs_f64());
                            t.take();
                            res[idx] = Some(r);
                            items -= 1;
                            return Some(());
                        }
                    }
                }
                None
            })
            .await;
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

pub async fn fully_mesh(sim: &mut Netsim<Command, Event>, timeout: Duration) -> Result<()> {
    for i in 0..sim.machines().len() {
        let machine = &mut sim.machines_mut()[i];
        loop {
            if let Some(Event::NewListenAddr(addr)) = future::timeout(Duration::from_secs(3), machine.recv()).await? {
                if !addr.is_loopback() {
                    let peer_id = machine.peer_id();
                    for machine in sim.machines_mut() {
                        machine.send(Command::AddAddress(peer_id, addr.clone()));
                    }
                    break;
                }
            }
        }
    }
    fully_meshed(sim, timeout).await
}

pub async fn fully_meshed<E>(sim: &mut Netsim<Command, E>, timeout: Duration) -> Result<()>
where
    E: Borrow<Event> + FromStr<Err = anyhow::Error> + Display + Send + 'static,
{
    let deadline = task::sleep(timeout);
    futures::pin_mut!(deadline);
    let peers = sim.machines().iter().map(|m| m.peer_id()).collect::<Vec<_>>();
    for (idx, machine) in sim.machines_mut().iter_mut().enumerate() {
        let mut peers = peers
            .iter()
            .filter(|p| **p != peers[idx])
            .copied()
            .collect::<BTreeSet<_>>();
        while !peers.is_empty() {
            let res = {
                let f = machine.select(|ev| m!(ev.borrow(), Event::Connected(p) => *p));
                futures::pin_mut!(f);
                match select(deadline.as_mut(), f).await {
                    Either::Left(_) => Either::Left(()),
                    Either::Right(r) => Either::Right(r),
                }
            };
            match res {
                Either::Left(_) => {
                    for e in machine.drain() {
                        tracing::error!("got event {}", e);
                    }
                    bail!(
                        "fully_meshed timed out after {:.1}sec ({}, {:?})",
                        timeout.as_secs_f64(),
                        idx,
                        peers
                    )
                }
                Either::Right((None, _)) => bail!("got not peer"),
                Either::Right((Some(p), _)) => {
                    peers.remove(&p);
                }
            }
        }
    }
    Ok(())
}
