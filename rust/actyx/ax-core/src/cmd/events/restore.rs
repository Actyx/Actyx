use super::dump::Diag;
use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    crypto::KeyPair,
    node_connection::request_banyan,
    private_key::{load_dev_cert, AxPrivateKey},
    util::{
        formats::{
            banyan_protocol::{decode_dump_header, BanyanRequest, BanyanResponse},
            ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt,
        },
        gen_stream::GenStream,
    },
};
use cbor_data::{Cbor, CborBuilder, Encoder};
use futures::Stream;
use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    path::PathBuf,
};
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

#[derive(clap::Parser, Clone, Debug)]
/// restore events from an event dump to a temporary topic
pub struct RestoreOpts {
    /// file to read the dump from
    #[arg(long, short = 'I', value_name = "FILE")]
    input: Option<PathBuf>,
    #[command(flatten)]
    console_opt: ConsoleOpt,
    /// suppress progress information on stderr
    #[arg(long, short)]
    quiet: bool,
    /// load dump via the cloud and store it as the given filename
    #[arg(long, value_name = "FILE")]
    cloud: Option<PathBuf>,
    /// location to read developer certificate from
    #[arg(long, value_name = "FILE")]
    cert: Option<PathBuf>,
    /// base URL where to find the cloudmirror (only for --cloud)
    /// defaults to wss://cloudmirror.actyx.net/forward
    #[arg(long, value_name = "URL")]
    url: Option<String>,
}
pub const URL: &str = "wss://cloudmirror.actyx.net/forward";

trait IO {
    type Out;
    fn io(self, ctx: impl AsRef<str>) -> ActyxOSResult<Self::Out>;
}
impl<T, E: std::fmt::Display> IO for Result<T, E> {
    type Out = T;
    fn io(self, ctx: impl AsRef<str>) -> ActyxOSResult<Self::Out> {
        self.map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("{}: {}", ctx.as_ref(), e)))
    }
}

trait BR {
    fn br(self) -> ActyxOSResult<()>;
}
impl BR for BanyanResponse {
    fn br(self) -> ActyxOSResult<()> {
        match self {
            BanyanResponse::Ok => Ok(()),
            BanyanResponse::Error(e) => Err(ActyxOSError::new(
                ActyxOSCode::ERR_IO,
                format!("error from Actyx node: {}", e),
            )),
            BanyanResponse::Future => Err(ActyxOSError::new(
                ActyxOSCode::ERR_IO,
                "message from Actyx node from the future",
            )),
        }
    }
}

pub struct EventsRestore;
impl AxCliCommand for EventsRestore {
    type Opt = RestoreOpts;
    type Output = ();

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(GenStream::new(move |_co| async move {
            if opts.cloud.is_some() && opts.input.is_some() {
                return Err(ActyxOSError::new(
                    ActyxOSCode::ERR_UNSUPPORTED,
                    "cannot restore from cloud and file at the same time",
                ));
            }

            let mut diag = Diag::new(opts.quiet);

            let mut input: Box<dyn Read> = if let Some(ref input) = opts.input {
                Box::new(File::open(input.as_path()).io("opening input dump")?)
            } else if let Some(ref cloud) = opts.cloud {
                let file = File::create(cloud.as_path()).io("opening cloud dump")?;
                let cert =
                    load_dev_cert(opts.cert).ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "cannot read dev cert")?;
                let private_key = cert.private_key().map(ActyxOSResult::Ok).unwrap_or_else(|| {
                    Ok(AxPrivateKey::from_file(AxPrivateKey::default_user_identity_path()?)?.to_private())
                })?;
                let url = opts.url.unwrap_or_else(|| URL.to_owned());
                diag.log(format!("connecting to {}", url))?;
                let mut ws = connect(URL).io("opening websocket")?.0;
                let msg = ws.read_message().io("read token message")?;
                if let Message::Text(token) = msg {
                    let signature = KeyPair::from(private_key).sign(token.as_bytes());
                    let response = CborBuilder::new().encode_array(|b| {
                        b.encode_bytes(signature);
                        b.encode_str(serde_json::to_string(&cert.manifest_dev_cert()).unwrap());
                    });
                    ws.write_message(Message::Binary(response.as_slice().into()))
                        .io("write signature message")?;
                    let ok = ws.read_message().io("read ok message")?;
                    if ok != Message::Text("OK".into()) {
                        return Err(ActyxOSError::new(ActyxOSCode::ERR_UNAUTHORIZED, ok.to_string()));
                    }
                    eprintln!("connection open, waiting for dump");
                    eprintln!("now start `ax events dump --cloud {}` on the source machine", token);
                } else {
                    return Err(ActyxOSError::new(
                        ActyxOSCode::ERR_INVALID_INPUT,
                        "received wrong message from server",
                    ));
                }
                Box::new(WsRead::new(file, ws))
            } else {
                Box::new(std::io::stdin())
            };

            let mut buf = Vec::new();
            buf.resize(100_000, 0u8);
            let mut pos = 0;
            let mut decoder = zstd::stream::write::Decoder::new(Vec::new()).io("starting decoder")?;
            let (node_id, topic, timestamp) = loop {
                let len = input.read(&mut buf.as_mut_slice()[pos..]).io("reading dump")?;
                diag.log(format!("received {} bytes", len))?;

                decoder
                    .write_all(&buf.as_slice()[pos..pos + len])
                    .io("decoding header")?;
                decoder.flush().io("flushing header")?;
                pos += len;

                match Cbor::checked_prefix(&decoder.get_ref().as_slice()[..pos]) {
                    Ok((cbor, _rest)) => {
                        break decode_dump_header(cbor).ok_or_else(|| {
                            ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, "cannot read dump header")
                        })?
                    }
                    Err(e) => {
                        if len == 0 || pos == buf.len() {
                            return Err(ActyxOSError::new(
                                ActyxOSCode::ERR_INVALID_INPUT,
                                format!("cannot read dump header: {}", e),
                            ));
                        }
                    }
                }
            };

            // keep the bytes in the buffer because the Actyx node will need to read the header as well

            diag.log(format!("sending dump from node {} topic `{}`", node_id, topic))?;
            let topic = format!("dump-{}", timestamp.to_rfc3339()).replace(':', "-");
            diag.log(format!("uploading to topic `{}`", topic))?;

            let (mut conn, peer) = opts.console_opt.connect().await?;

            request_banyan(&mut conn, peer, BanyanRequest::MakeFreshTopic(topic.clone())).await?;
            let mut count = 0;
            loop {
                request_banyan(
                    &mut conn,
                    peer,
                    BanyanRequest::AppendEvents(topic.clone(), buf[..pos].into()),
                )
                .await?;
                count += pos;
                diag.status(format!("{} bytes uploaded", count))?;
                pos = input.read(buf.as_mut_slice()).io("reading dump")?;
                if pos == 0 {
                    break;
                }
            }
            diag.log(format!("in total {} bytes uploaded", count))?;
            request_banyan(&mut conn, peer, BanyanRequest::Finalise(topic.clone())).await?;
            diag.log(format!("topic switched to `{}`", topic))?;
            diag.log("Actyx node switched into read-only network mode")?;

            Ok(())
        }))
    }

    fn pretty(_result: Self::Output) -> String {
        String::new()
    }
}

struct WsRead {
    file: File,
    sock: WebSocket<MaybeTlsStream<TcpStream>>,
    buf: Vec<u8>,
    pos: usize,
}

impl WsRead {
    fn new(file: File, sock: WebSocket<MaybeTlsStream<TcpStream>>) -> Self {
        Self {
            file,
            sock,
            buf: Vec::new(),
            pos: 0,
        }
    }
}
impl Read for WsRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while self.pos >= self.buf.len() {
            if !self.sock.can_read() {
                return Ok(0);
            }
            let msg = match self.sock.read_message() {
                Ok(msg) => msg,
                Err(tungstenite::Error::ConnectionClosed) => return Ok(0),
                Err(e) => return Err(std::io::Error::new(ErrorKind::Other, e)),
            };
            if let Message::Binary(b) = msg {
                self.buf = b;
                self.pos = 0;
                self.file
                    .write_all(self.buf.as_slice())
                    .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
            }
        }
        let bytes = (self.buf.len() - self.pos).min(buf.len());
        buf[..bytes].copy_from_slice(&self.buf[self.pos..self.pos + bytes]);
        self.pos += bytes;
        Ok(bytes)
    }
}
impl Drop for WsRead {
    fn drop(&mut self) {
        self.file.flush().ok();
    }
}
