use anyhow::Context;
use futures::{future::BoxFuture, AsyncRead, FutureExt};
use ipfs_embed::multiaddr::Protocol;
use libp2p::{
    core::{
        either::{EitherOutput, EitherTransport},
        muxing::StreamMuxerBox,
        transport::{Boxed, MemoryTransport},
        upgrade::AuthenticationVersion,
        ConnectedPoint,
    },
    dns::{ResolverConfig, TokioDnsConfig},
    identity, noise,
    plaintext::PlainText2Config,
    pnet::{PnetConfig, PreSharedKey},
    tcp::{tokio::TcpStream, TokioTcpConfig},
    websocket::{framed::Connection, wrap_connection, WsConfig},
    yamux::YamuxConfig,
    PeerId, Transport,
};
use libp2p_maybe_transport::{combined, MaybeUpgrade, UpgradeMaybe};
use soketto::handshake;
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
    let base_transport = combined::CombinedTransport::new(base_transport, WsConfig::new, maybe_upgrade, |mut addr| {
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

#[derive(Clone)]
struct Upgrader;
type Tcp = TokioDnsConfig<TokioTcpConfig>;
impl UpgradeMaybe<Tcp, WsConfig<Tcp>> for Upgrader {
    type UpgradeFuture = BoxFuture<'static, Result<<WsConfig<Tcp> as Transport>::Output, <Tcp as Transport>::Output>>;

    fn try_upgrade(inner: TcpStream) -> Self::UpgradeFuture {
        async move {
            let mut buffer = [0; 3];
            if inner.0.peek(&mut buffer).await.is_ok() && buffer == *b"GET" {
                tracing::info!("It's probably HTTP :-)");
                let stream = inner; //upgrade.map_err(Error::Transport).await?;
                tracing::debug!("incoming connection from"); // {}", remote1);

                let stream = EitherOutput::Second(stream);
                tracing::debug!("receiving websocket handshake request"); //, remote2);

                let mut server = handshake::Server::new(stream);

                //                    if use_deflate {
                //                        server.add_extension(Box::new(Deflate::new(connection::Mode::Server)));
                //                    }

                let ws_key = {
                    let request = server
                        .receive_request()
                        //.map_err(|e| Error::Handshake(Box::new(e)))
                        .await
                        .expect("FIXME");
                    request.into_key()
                };

                tracing::debug!("accepting websocket handshake request"); // from {}", remote2);

                let response = handshake::server::Response::Accept {
                    key: &ws_key,
                    protocol: None,
                };

                server
                    .send_response(&response)
                    //.map_err(|e| Error::Handshake(Box::new(e)))
                    .await
                    .expect("FIXME");

                let conn = {
                    let builder = server.into_builder();
                    //                        builder.set_max_message_size(max_size);
                    //                        builder.set_max_frame_size(max_size);
                    Connection::new(builder)
                };

                Ok(wrap_connection(
                    conn,
                    // unused FIXME rm
                    ConnectedPoint::Dialer {
                        address: "/ip4/127.0.0.1/tcp/4242".parse().unwrap(),
                    },
                ))
            } else {
                Err(inner)
            }
        }
        .boxed()
    }
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
