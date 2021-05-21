use actyxos_sdk::{language::Query, Payload, StreamNr, TagSet};
use anyhow::Result;
use crypto::{KeyPair, PrivateKey};
pub use libp2p::{multiaddr, Multiaddr, PeerId};
use std::{net::SocketAddr, path::PathBuf};
use structopt::StructOpt;
use swarm::SwarmConfig;
use trees::axtrees::AxKey;

#[derive(Clone, Debug, StructOpt)]
pub struct Config {
    #[structopt(long)]
    pub path: Option<PathBuf>,
    #[structopt(long)]
    pub node_name: Option<String>,
    #[structopt(long)]
    pub keypair: u64,
    #[structopt(long)]
    pub enable_mdns: bool,
    #[structopt(long)]
    pub enable_discovery: bool,
    #[structopt(long)]
    pub enable_metrics: bool,
    #[structopt(long)]
    pub enable_fast_path: bool,
    #[structopt(long)]
    pub enable_slow_path: bool,
    #[structopt(long)]
    pub enable_root_map: bool,
    #[structopt(long)]
    pub listen_on: Vec<Multiaddr>,
    #[structopt(long)]
    pub bootstrap: Vec<Multiaddr>,
    #[structopt(long)]
    pub external: Vec<Multiaddr>,
    #[structopt(long)]
    pub enable_api: Option<SocketAddr>,
}

impl From<Config> for async_process::Command {
    fn from(config: Config) -> Self {
        let swarm_cli = target_dir().join("swarm-cli");
        if !swarm_cli.exists() {
            panic!("failed to find the swarm-cli binary at {}", swarm_cli.display());
        }
        let mut cmd = Self::new(swarm_cli);
        if let Some(path) = config.path.as_ref() {
            cmd.arg("--path").arg(path);
        }
        if let Some(node_name) = config.node_name.as_ref() {
            cmd.arg("--node-name").arg(node_name);
        } else {
            cmd.arg("--node-name").arg(format!("node{}", config.keypair));
        }
        cmd.arg("--keypair").arg(config.keypair.to_string());
        for listen_on in &config.listen_on {
            cmd.arg("--listen-on").arg(listen_on.to_string());
        }
        for bootstrap in &config.bootstrap {
            cmd.arg("--bootstrap").arg(bootstrap.to_string());
        }
        for external in &config.external {
            cmd.arg("--external").arg(external.to_string());
        }
        if config.enable_mdns {
            cmd.arg("--enable-mdns");
        }
        if config.enable_discovery {
            cmd.arg("--enable-discovery");
        }
        if config.enable_metrics {
            cmd.arg("--enable-metrics");
        }
        if config.enable_fast_path {
            cmd.arg("--enable-fast-path");
        }
        if config.enable_slow_path {
            cmd.arg("--enable-slow-path");
        }
        if config.enable_root_map {
            cmd.arg("--enable-root-map");
        }
        if let Some(api) = config.enable_api {
            cmd.arg("--enable-api").arg(api.to_string());
        }
        cmd
    }
}

impl From<Config> for SwarmConfig {
    fn from(config: Config) -> Self {
        Self {
            db_path: config.path,
            node_name: config.node_name,
            keypair: Some(keypair(config.keypair)),
            enable_mdns: config.enable_mdns,
            topic: "swarm-cli".into(),
            listen_addresses: config.listen_on,
            bootstrap_addresses: config.bootstrap,
            external_addresses: config.external,
            enable_fast_path: config.enable_fast_path,
            enable_slow_path: config.enable_slow_path,
            enable_root_map: config.enable_root_map,
            enable_discovery: config.enable_discovery,
            enable_metrics: config.enable_metrics,
            ..Default::default()
        }
    }
}

pub fn keypair(i: u64) -> KeyPair {
    let mut keypair = [0; 32];
    keypair[..8].copy_from_slice(&i.to_be_bytes());
    KeyPair::from(PrivateKey::from_bytes(&keypair).unwrap())
}

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    AddAddress(PeerId, Multiaddr),
    Append(StreamNr, Vec<(TagSet, Payload)>),
    SubscribeQuery(Query),
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::AddAddress(peer, addr) => write!(f, ">add-address {} {}", peer, addr)?,
            Self::Append(nr, events) => write!(f, ">append {} {}", nr, serde_json::to_string(events).unwrap())?,
            Self::SubscribeQuery(expr) => write!(f, ">query {}", expr)?,
        }
        Ok(())
    }
}

impl std::str::FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split_whitespace();
        Ok(match parts.next() {
            Some(">add-address") => {
                let peer: PeerId = parts.next().unwrap().parse()?;
                let addr: Multiaddr = parts.next().unwrap().parse()?;
                Self::AddAddress(peer, addr)
            }
            Some(">query") => Self::SubscribeQuery(s.split_at(7).1.parse()?),
            Some(">append") => {
                let s = s.split_at(8).1;
                let mut iter = s.splitn(2, ' ');
                let nr: u64 = iter.next().unwrap().parse()?;
                let events = serde_json::from_str(iter.next().unwrap())?;
                Self::Append(nr.into(), events)
            }
            _ => {
                return Err(anyhow::anyhow!("invalid command '{}'", s));
            }
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    NewListenAddr(Multiaddr),
    ExpiredListenAddr(Multiaddr),
    NewExternalAddr(Multiaddr),
    ExpiredExternalAddr(Multiaddr),
    Connected(PeerId),
    Disconnected(PeerId),
    Subscribed(PeerId, String),
    Result((u64, AxKey, Payload)),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NewListenAddr(multiaddr) => {
                write!(f, "<new-listen-addr {}", multiaddr)?;
            }
            Self::ExpiredListenAddr(multiaddr) => {
                write!(f, "<expired-listen-addr {}", multiaddr)?;
            }
            Self::NewExternalAddr(addr) => {
                write!(f, "<new-external-addr {}", addr)?;
            }
            Self::ExpiredExternalAddr(addr) => {
                write!(f, "<expired-external-addr {}", addr)?;
            }
            Self::Connected(peer_id) => {
                write!(f, "<connected {}", peer_id)?;
            }
            Self::Disconnected(peer_id) => {
                write!(f, "<disconnected {}", peer_id)?;
            }
            Self::Subscribed(peer_id, topic) => {
                write!(f, "<subscribed {} {}", peer_id, topic)?;
            }
            Self::Result(res) => {
                write!(f, "<result {}", serde_json::to_string(res).unwrap())?;
            }
        }
        Ok(())
    }
}

impl std::str::FromStr for Event {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split_whitespace();
        Ok(match parts.next() {
            Some("<new-listen-addr") => Self::NewListenAddr(parts.next().unwrap().parse()?),
            Some("<expired-listen-addr") => Self::ExpiredListenAddr(parts.next().unwrap().parse()?),
            Some("<new-external-addr") => Self::NewExternalAddr(parts.next().unwrap().parse()?),
            Some("<expired-external-addr") => Self::ExpiredExternalAddr(parts.next().unwrap().parse()?),
            Some("<connected") => Self::Connected(parts.next().unwrap().parse()?),
            Some("<disconnected") => Self::Disconnected(parts.next().unwrap().parse()?),
            Some("<subscribed") => {
                let peer_id = parts.next().unwrap().parse()?;
                let topic = parts.next().unwrap();
                Self::Subscribed(peer_id, topic.into())
            }
            Some("<result") => {
                let json: String = parts.collect();
                Self::Result(serde_json::from_str(&json)?)
            }
            _ => {
                return Err(anyhow::anyhow!("invalid event '{}'", s));
            }
        })
    }
}

fn target_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::tags;

    #[test]
    fn test_command() -> Result<()> {
        let command = &[
            Command::Append(
                42.into(),
                vec![(tags!("a", "b"), Payload::from_json_str("{}").unwrap())],
            ),
            Command::SubscribeQuery("FROM 'a' & 'b' | 'c'".parse().unwrap()),
        ];
        for cmd in command.iter() {
            let cmd2: Command = cmd.to_string().parse()?;
            assert_eq!(cmd, &cmd2);
        }
        Ok(())
    }

    #[test]
    fn test_event() -> Result<()> {
        let event = &[Event::Result((
            0,
            AxKey::new(tags!(), 0, 0),
            Payload::from_json_str("{}").unwrap(),
        ))];
        for ev in event.iter() {
            let ev2: Event = ev.to_string().parse()?;
            assert_eq!(ev, &ev2);
        }
        Ok(())
    }
}
