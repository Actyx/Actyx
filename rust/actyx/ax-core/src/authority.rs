use std::str::FromStr;

use crate::util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult};
use libp2p::{multiaddr::Protocol, Multiaddr};
use std::net::ToSocketAddrs;

#[derive(Debug, Clone)]
pub struct Authority {
    pub original: String,
    pub addrs: Vec<Multiaddr>,
}

impl FromStr for Authority {
    type Err = ActyxOSError;

    fn from_str(s: &str) -> ActyxOSResult<Self> {
        let original = s.to_owned();
        if let Ok(m) = s.parse::<Multiaddr>() {
            Ok(Self {
                original,
                addrs: vec![m],
            })
        } else if let Ok(s) = s.to_socket_addrs() {
            Ok(Self {
                original,
                addrs: s
                    .map(|a| Multiaddr::empty().with(a.ip().into()).with(Protocol::Tcp(a.port())))
                    .collect(),
            })
        } else if let Ok(s) = (s, 4458).to_socket_addrs() {
            Ok(Self {
                original,
                addrs: s
                    .map(|a| Multiaddr::empty().with(a.ip().into()).with(Protocol::Tcp(a.port())))
                    .collect(),
            })
        } else {
            Err(ActyxOSError::new(
                ActyxOSCode::ERR_INVALID_INPUT,
                format!("cannot interpret {} as address", original),
            ))
        }
    }
}
