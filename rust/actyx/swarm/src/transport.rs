use libp2p::{
    core::{
        either::EitherTransport,
        muxing::StreamMuxerBox,
        transport::{Boxed, MemoryTransport},
        upgrade::Version,
    },
    dns::DnsConfig,
    identity, noise,
    plaintext::PlainText2Config,
    pnet::{PnetConfig, PreSharedKey},
    tcp::TokioTcpConfig,
    yamux::YamuxConfig,
    PeerId, Transport,
};
use std::{io, time::Duration};

/// Builds the transport that serves as a common ground for all connections.
///
/// This transport is compatible with secio, but prefers noise encryption
pub async fn build_transport(
    key_pair: identity::Keypair,
    psk: Option<PreSharedKey>,
    upgrade_timeout: Duration,
) -> anyhow::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let base_transport = DnsConfig::system(TokioTcpConfig::new().nodelay(true)).await?;
    let maybe_encrypted = match psk {
        Some(psk) => {
            EitherTransport::Left(base_transport.and_then(move |socket, _| PnetConfig::new(psk).handshake(socket)))
        }
        None => EitherTransport::Right(base_transport),
    };
    let xx_keypair = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&key_pair)
        .unwrap();
    let noise_config = noise::NoiseConfig::xx(xx_keypair).into_authenticated();
    let yamux_config = YamuxConfig::default();
    let transport = maybe_encrypted
        .upgrade(Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .timeout(upgrade_timeout)
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .boxed();
    Ok(transport)
}

pub async fn build_dev_transport(
    key_pair: identity::Keypair,
    upgrade_timeout: Duration,
) -> anyhow::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let plaintext_config = PlainText2Config {
        local_public_key: key_pair.public(),
    };
    let yamux_config = YamuxConfig::default();
    let transport = MemoryTransport {}
        .upgrade(Version::V1)
        .authenticate(plaintext_config)
        .multiplex(yamux_config)
        .timeout(upgrade_timeout)
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .boxed();
    Ok(transport)
}
