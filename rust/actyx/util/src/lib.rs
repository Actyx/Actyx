#![deny(clippy::future_not_send)]

pub mod base64_blob;
pub mod build;
pub mod formats;
pub mod immutable_sync;
pub mod keepalivestream3;
pub mod offsetmap_or_max;
pub mod pinned_resource;
pub mod pinned_resource_sync;
pub mod reentrant_safe_mutex;
pub mod serde_support;
pub mod serde_util;
pub mod tracing_set_log_level;
pub mod value_or_limit;
pub mod version;
pub mod wrapping_subscriber;

pub use self::value_or_limit::*;
pub use tracing_set_log_level::*;

use anyhow::bail;
use multiaddr::{Multiaddr, Protocol};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use std::{convert::TryFrom, net::IpAddr};
use std::{
    fmt::{Display, Formatter},
    vec,
};
use tracing_subscriber::EnvFilter;

/// Sets up a logging and a panic handler that logs panics.
pub fn setup_logger() {
    tracing_log::LogTracer::init().ok();
    let env = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| "info".to_owned());
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new(env))
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();
    log_panics::init();
}

#[derive(Clone, Debug, PartialEq)]
pub struct SocketAddrHelper {
    inner: HashSet<SocketAddr>,
}
impl TryFrom<Multiaddr> for SocketAddrHelper {
    type Error = anyhow::Error;
    fn try_from(mut multi_addr: Multiaddr) -> Result<Self, Self::Error> {
        if let Some(Protocol::Tcp(port)) = multi_addr.pop() {
            let inner: HashSet<SocketAddr> = match multi_addr.pop() {
                Some(Protocol::Ip4(ip4)) => (ip4, port).to_socket_addrs()?.collect(),
                Some(Protocol::Dns4(dns4)) => (dns4.to_string(), port).to_socket_addrs()?.collect(),
                Some(Protocol::Ip6(ip6)) => (ip6, port).to_socket_addrs()?.collect(),
                Some(Protocol::Dns6(dns6)) => (dns6.to_string(), port).to_socket_addrs()?.collect(),
                Some(e) => {
                    bail!("Unexpected multiaddr protocol \"{:?}\"", e)
                }
                None => {
                    bail!("Invalid multiaddr, only {ip4,dns4,ip6,dns6} supported")
                }
            };

            if multi_addr.pop().is_some() {
                bail!("Nested protocols are not supported");
            }
            Ok(Self { inner })
        } else {
            bail!("Multiaddress must end with tcp")
        }
    }
}

impl SocketAddrHelper {
    // Parses common multiaddrs and resolves dns4 to ip4 hosts.
    // Limitations: No nested protocols, only tcp.
    pub fn parse_multiaddr(multiaddr_str: &str) -> anyhow::Result<Self> {
        let multiaddr: Multiaddr = multiaddr_str.parse()?;
        SocketAddrHelper::try_from(multiaddr)
    }

    fn from_host_string(host_string: &str) -> anyhow::Result<Self> {
        let inner = host_string.to_socket_addrs()?.collect();
        Ok(Self { inner })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.set_port(port);
        self
    }
    pub fn set_port(&mut self, port: u16) {
        self.inner = self
            .inner
            .drain()
            .map(|mut x| {
                x.set_port(port);
                x
            })
            .collect();
    }

    /// Takes an input string, which can either be a port, or a host:port
    /// combination. If only a port is given, the provided `default_host` will be
    /// prepended.
    pub fn from_port(port_string: &str, default_host: &str) -> anyhow::Result<Self> {
        if let Ok(port) = port_string.parse::<u16>() {
            let inner = (default_host, port).to_socket_addrs()?.collect();
            Ok(Self { inner })
        } else {
            let inner = port_string.to_socket_addrs()?.collect();
            Ok(Self { inner })
        }
    }

    /// Takes an input string, which can either be a host, or a host:port
    /// combination. If only a host is given, the provided `default_port` will be
    /// appended.
    pub fn from_host(host_string: &str, default_port: u16) -> anyhow::Result<Self> {
        if let Ok(addr) = host_string.parse() {
            Ok(addr)
        } else {
            format!("{}:{}", host_string, default_port).parse()
        }
    }

    pub fn append(&mut self, other: Self) {
        self.inner.extend(other.inner.into_iter());
    }

    pub fn to_multiaddrs(&self) -> vec::IntoIter<Multiaddr> {
        let v: Vec<Multiaddr> = self.inner.iter().cloned().map(to_multiaddr).collect();
        v.into_iter()
    }

    pub fn unspecified(port: u16) -> Self {
        let ipv6: Self = format!("[::]:{}", port).parse().unwrap();
        let ipv4: Self = format!("0.0.0.0:{}", port).parse().unwrap();
        let inner = ipv6.into_iter().chain(ipv4.into_iter()).collect();
        Self { inner }
    }
}
impl IntoIterator for SocketAddrHelper {
    type Item = SocketAddr;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
impl<'a> IntoIterator for &'a SocketAddrHelper {
    type Item = &'a SocketAddr;
    type IntoIter = std::collections::hash_set::Iter<'a, SocketAddr>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
impl FromStr for SocketAddrHelper {
    type Err = anyhow::Error;
    fn from_str(str: &str) -> anyhow::Result<Self> {
        Self::from_host_string(str).or_else(|_| Self::parse_multiaddr(str))
    }
}
impl Display for SocketAddrHelper {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let v = self
            .inner
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{}]", v)
    }
}

impl<'de> Deserialize<'de> for SocketAddrHelper {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<SocketAddrHelper, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

fn to_multiaddr(socket_addr: SocketAddr) -> Multiaddr {
    let proto_ip = match socket_addr.ip() {
        IpAddr::V4(ip4) => Protocol::Ip4(ip4),
        IpAddr::V6(ip6) => Protocol::Ip6(ip6),
    };
    Multiaddr::empty()
        .with(proto_ip)
        .with(Protocol::Tcp(socket_addr.port()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_multiaddr() {
        let str = ("/ip4/127.0.0.1/tcp/5001").to_owned();
        let ret = SocketAddrHelper::from_str(&str).unwrap();
        let mut inner = HashSet::new();
        inner.insert("127.0.0.1:5001".parse().unwrap());
        assert_eq!(ret, SocketAddrHelper { inner });

        let str = ("/dns4/localhost/tcp/5001").to_owned();
        let _ = SocketAddrHelper::from_str(&str).unwrap();
    }

    #[test]
    fn should_work_with_localhost() {
        let str = "localhost:4242";
        let _ = SocketAddrHelper::from_host_string(str).unwrap();
    }

    #[test]
    fn should_work_with_unspecified() {
        let vec = SocketAddrHelper::unspecified(4242);
        for i in vec {
            assert!(i.ip().is_unspecified());
        }
    }
}
