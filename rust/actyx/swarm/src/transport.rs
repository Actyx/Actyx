use anyhow::Context;
use futures::{future::BoxFuture, FutureExt};
use ipfs_embed::multiaddr::Protocol;
use libp2p::{
    core::{
        either::EitherTransport,
        muxing::StreamMuxerBox,
        transport::{Boxed, MemoryTransport},
        upgrade::AuthenticationVersion,
    },
    dns::{ResolverConfig, TokioDnsConfig},
    identity, noise,
    plaintext::PlainText2Config,
    pnet::{PnetConfig, PreSharedKey},
    tcp::{tokio::TcpStream, TokioTcpConfig},
    websocket::WsConfig,
    yamux::YamuxConfig,
    PeerId, Transport,
};
use libp2p_combined_transport::CombinedTransport;
use std::{io, time::Duration};

fn maybe_upgrade(r: TcpStream) -> BoxFuture<'static, Result<TcpStream, TcpStream>> {
    async move {
        let mut buffer = [0; 3];
        if r.0.peek(&mut buffer).await.is_ok() && buffer == *b"GET" {
            tracing::info!("It's probably HTTP :-)");
            Ok(r)
        } else {
            Err(r)
        }
    }
    .boxed()
}

/// Builds the transport that serves as a common ground for all connections.
///
/// This transport is compatible with secio, but prefers noise encryption
pub async fn build_transport(
    key_pair: identity::Keypair,
    psk: Option<PreSharedKey>,
    upgrade_timeout: Duration,
) -> anyhow::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let tcp = TokioTcpConfig::new().nodelay(true);
    let base_transport = if cfg!(target_os = "android") {
        // No official support for DNS on Android.
        // see https://github.com/Actyx/Cosmos/issues/6582
        TokioDnsConfig::custom(tcp, ResolverConfig::cloudflare(), Default::default())
            .context("Creating TokioDnsConfig")?
    } else {
        TokioDnsConfig::system(tcp).context("Creating TokioDnsConfig")?
    };
    let base_transport = CombinedTransport::new(base_transport, WsConfig::new, maybe_upgrade, |mut addr| {
        addr.push(Protocol::Ws("/".into()));
        addr
    });
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
        .upgrade()
        .authenticate_with_version(noise_config, AuthenticationVersion::V1SimultaneousOpen)
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
        .upgrade()
        .authenticate_with_version(plaintext_config, AuthenticationVersion::V1SimultaneousOpen)
        .multiplex(yamux_config)
        .timeout(upgrade_timeout)
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .boxed();
    Ok(transport)
}
