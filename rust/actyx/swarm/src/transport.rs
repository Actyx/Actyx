use anyhow::Context;
use libp2p::{
    core::{either::EitherTransport, muxing::StreamMuxerBox, transport::Boxed, upgrade::Version},
    dns::{ResolverConfig, TokioDnsConfig},
    identity, noise,
    pnet::{PnetConfig, PreSharedKey},
    tcp::{GenTcpConfig, TokioTcpTransport},
    yamux::YamuxConfig,
    PeerId, Transport,
};
use std::{io, time::Duration};

/// Builds the transport that serves as a common ground for all connections.
pub async fn build_transport(
    key_pair: identity::Keypair,
    psk: Option<PreSharedKey>,
    upgrade_timeout: Duration,
) -> anyhow::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let tcp = TokioTcpTransport::new(GenTcpConfig::new().nodelay(true));
    let base_transport = if cfg!(target_os = "android") {
        // No official support for DNS on Android.
        // see https://github.com/Actyx/Cosmos/issues/6582
        TokioDnsConfig::custom(tcp, ResolverConfig::cloudflare(), Default::default())
            .context("Creating TokioDnsConfig")?
    } else {
        match trust_dns_resolver::system_conf::read_system_conf() {
            Ok((cfg, opts)) => TokioDnsConfig::custom(tcp, cfg, opts).context("Creating TokioDnsConfig")?,
            Err(e) => {
                tracing::warn!(
                    "falling back to Cloudflare DNS since parsing system settings failed with {:#}",
                    e
                );
                TokioDnsConfig::custom(tcp, ResolverConfig::cloudflare(), Default::default())
                    .context("Creating TokioDnsConfig")?
            }
        }
    };
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
