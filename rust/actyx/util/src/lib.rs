#![deny(clippy::future_not_send)]

pub mod base64_blob;
pub mod build;
pub mod chunk_unless_pending;
pub mod drainer;
pub mod formats;
pub mod gen_stream;
pub mod immutable_sync;
pub mod keepalivestream3;
pub mod pinned_resource;
pub mod pinned_resource_sync;
pub mod reentrant_safe_mutex;
pub mod serde_support;
pub mod serde_util;
pub mod trace_poll;
pub mod tracing_set_log_level;
pub mod value_or_limit;
pub mod version;

pub use self::value_or_limit::*;
pub use tracing_set_log_level::*;

use anyhow::bail;
use multiaddr::{Multiaddr, Protocol};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::iter::FromIterator;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::num::NonZeroU16;
use std::str::FromStr;
use std::{convert::TryFrom, net::IpAddr};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// Sets up a logging and a panic handler that logs panics.
pub fn setup_logger() {
    tracing_log::LogTracer::init().ok();
    let env = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| "info".to_owned());
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::new(env))
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();
    log_panics::init();
}

#[derive(Clone, Debug, PartialEq)]
pub struct SocketAddrHelper {
    inner: HashSet<SocketAddr>,
}

impl SocketAddrHelper {
    pub fn empty() -> Self {
        Self { inner: HashSet::new() }
    }

    // Parses common multiaddrs and resolves dns4 to ip4 hosts.
    // Limitations: No nested protocols, only tcp.
    pub fn parse_multiaddr(multiaddr_str: &str) -> anyhow::Result<Self> {
        let multiaddr: Multiaddr = multiaddr_str.parse()?;
        SocketAddrHelper::try_from(multiaddr)
    }

    pub fn from_host_string(host_string: &str) -> anyhow::Result<Self> {
        let inner = host_string.to_socket_addrs()?.collect();
        Ok(Self { inner })
    }

    /// Takes an input string, which can either be a host, or a host:port
    /// combination. If only a host is given, the provided `default_port` will be
    /// appended.
    pub fn from_host(host_string: &str, default_port: NonZeroU16) -> anyhow::Result<Self> {
        if let Ok(addr) = host_string.parse() {
            Ok(addr)
        } else {
            Ok(Self {
                inner: (host_string, default_port.into()).to_socket_addrs()?.collect(),
            })
        }
    }

    pub fn from_ip_port(ip: IpAddr, port: u16) -> anyhow::Result<Self> {
        Ok(Self {
            inner: (ip, port).to_socket_addrs()?.collect(),
        })
    }

    pub fn append(&mut self, other: Self) {
        self.inner.extend(other.inner.into_iter());
    }

    pub fn to_multiaddrs(&self) -> impl Iterator<Item = Multiaddr> {
        self.inner.clone().into_iter().map(to_multiaddr)
    }

    pub fn unspecified(port: u16) -> anyhow::Result<Self> {
        let ipv6 = (Ipv6Addr::UNSPECIFIED, port)
            .to_socket_addrs()
            .expect("IPv6 Any:port should work");
        let ipv4 = (Ipv4Addr::UNSPECIFIED, port)
            .to_socket_addrs()
            .expect("IPv4 Any:port should work");
        let inner = ipv6.chain(ipv4).collect();
        Ok(Self { inner })
    }

    pub fn inject_bound_addr(&mut self, mut listen_addr: SocketAddr, bound_addr: SocketAddr) -> Option<()> {
        if listen_addr.port() != 0 {
            return None;
        }
        self.inner.remove(&listen_addr);
        listen_addr.set_port(bound_addr.port());
        self.inner.insert(listen_addr);
        Some(())
    }

    pub fn iter(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.into_iter().copied()
    }
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
                    bail!("Invalid multiaddr, only {{ip4,dns4,ip6,dns6}} supported")
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

impl From<SocketAddr> for SocketAddrHelper {
    fn from(s: SocketAddr) -> Self {
        let mut inner = HashSet::new();
        inner.insert(s);
        Self { inner }
    }
}

impl FromStr for SocketAddrHelper {
    type Err = anyhow::Error;
    fn from_str(str: &str) -> anyhow::Result<Self> {
        Self::from_host_string(str).or_else(|_| Self::parse_multiaddr(str))
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

impl FromIterator<SocketAddr> for SocketAddrHelper {
    fn from_iter<T: IntoIterator<Item = SocketAddr>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl Display for SocketAddrHelper {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let v = self
            .inner
            .iter()
            .map(SocketAddr::to_string)
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

pub fn to_socket_addr(m: Multiaddr) -> Option<SocketAddr> {
    let mut iter = m.iter();
    let ip = match iter.next() {
        Some(Protocol::Ip4(ip)) => IpAddr::V4(ip),
        Some(Protocol::Ip6(ip)) => IpAddr::V6(ip),
        _ => return None,
    };
    let port = match iter.next() {
        Some(Protocol::Tcp(p)) => p,
        Some(Protocol::Udp(p)) => p,
        _ => return None,
    };
    Some((ip, port).into())
}

pub fn to_multiaddr(socket_addr: SocketAddr) -> Multiaddr {
    let proto_ip = match socket_addr.ip() {
        IpAddr::V4(ip4) => Protocol::Ip4(ip4),
        IpAddr::V6(ip6) => Protocol::Ip6(ip6),
    };
    Multiaddr::empty()
        .with(proto_ip)
        .with(Protocol::Tcp(socket_addr.port()))
}

pub mod serde_str {
    //! Serializes fields annotated with `#[serde(with = "::util::serde_str")]` with their !
    //! `Display` implementation, deserializes fields using `FromStr`.
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
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
        let vec = SocketAddrHelper::unspecified(4242).unwrap();
        for i in vec {
            assert!(i.ip().is_unspecified());
        }
    }
}
