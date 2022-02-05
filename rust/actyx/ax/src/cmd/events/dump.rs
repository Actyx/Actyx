use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::{request_single, Task},
};
use actyx_sdk::{
    language::Query,
    service::{Order, QueryRequest},
};
use cbor_data::{value::Precision, CborBuilder, Encoder, Writer};
use chrono::{DateTime, Duration, Local, Utc};
use console::{user_attended_stderr, Term};
use futures::{channel::mpsc::channel, SinkExt, Stream, StreamExt};
use itertools::Itertools;
use std::{
    fs::File,
    io::{ErrorKind, Write},
    net::TcpStream,
    path::PathBuf,
};
use structopt::StructOpt;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};
use util::{
    formats::{
        events_protocol::{EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse,
    },
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// dump events described by an AQL query into a file
pub struct DumpOpts {
    #[structopt(name = "QUERY", required = true)]
    /// selection of event data to include in the dump
    query: String,
    #[structopt(long, short, value_name = "FILE")]
    /// file to write the dump to
    output: Option<PathBuf>,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    #[structopt(long, short)]
    /// suppress progress information on stderr
    quiet: bool,
    #[structopt(long, value_name = "TOKEN")]
    /// send dump via the cloud (start restore first to get the token)
    cloud: Option<String>,
    #[structopt(long, value_name = "URL")]
    /// base URL where to find the cloudmirror (only for --cloud)
    /// defaults to wss://cloudmirror.actyx.net/forward
    url: Option<String>,
}

pub(super) struct Diag {
    term: Option<Term>,
    status: Option<String>,
}
impl Diag {
    pub fn new(quiet: bool) -> Self {
        if quiet || !user_attended_stderr() {
            Self {
                term: None,
                status: None,
            }
        } else {
            Self {
                term: Some(Term::stderr()),
                status: None,
            }
        }
    }

    pub fn log(&mut self, s: impl AsRef<str>) -> ActyxOSResult<()> {
        self.do_log(s)
            .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error writing to terminal: {}", e)))
    }

    fn do_log(&mut self, s: impl AsRef<str>) -> anyhow::Result<()> {
        if let Some(ref mut term) = self.term {
            term.clear_line()?;
            term.write_line(s.as_ref())?;
            if let Some(ref status) = self.status {
                term.write_all(status.as_bytes())?;
                term.flush()?;
            }
        }
        Ok(())
    }

    pub fn status(&mut self, s: String) -> ActyxOSResult<()> {
        self.do_status(s)
            .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error writing to terminal: {}", e)))
    }

    fn do_status(&mut self, s: String) -> anyhow::Result<()> {
        if let Some(ref mut term) = self.term {
            term.clear_line()?;
            term.write_all(s.as_bytes())?;
            term.flush()?;
            self.status = Some(s);
        }
        Ok(())
    }
}
impl Drop for Diag {
    fn drop(&mut self) {
        if let Some(ref mut term) = self.term {
            term.clear_line().ok();
        }
    }
}

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

pub struct EventsDump;
impl AxCliCommand for EventsDump {
    type Opt = DumpOpts;
    type Output = ();

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(GenStream::new(move |_co| async move {
            let mut diag = Diag::new(opts.quiet);

            let query = opts
                .query
                .parse::<Query>()
                .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, format!("query invalid: {}", e)))?;

            let (mut conn, peer) = opts.console_opt.connect().await?;

            let mut out = zstd::Encoder::<Box<dyn Write>>::new(
                if let Some(ref out) = opts.output {
                    let file = File::create(out.as_path()).io("opening dump")?;
                    Box::new(file)
                } else if let Some(ref token) = opts.cloud {
                    let url = opts.url.clone().unwrap_or_else(|| super::restore::URL.to_owned());
                    let ws = connect(format!("{}/{}", url, token)).io("opening websocket")?.0;
                    Box::new(WsWrite::new(ws))
                } else {
                    Box::new(std::io::stdout())
                },
                21,
            )
            .io("initialising zstd")?;

            diag.log(format!(
                "connected to {} ({:?})",
                opts.console_opt.authority.original, opts.console_opt.authority.addrs
            ))?;

            let now = Local::now();

            let node_info = request_single(
                &mut conn,
                move |tx| Task::Admin(peer, AdminRequest::NodesLs, tx),
                |resp| match resp {
                    AdminResponse::NodesLsResponse(ls) => Ok(ls),
                    m => Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("got invalid response {:?}", m))),
                },
            )
            .await?;
            let node_details = request_single(
                &mut conn,
                move |tx| Task::Admin(peer, AdminRequest::NodesInspect, tx),
                |resp| match resp {
                    AdminResponse::NodesInspectResponse(ls) => Ok(ls),
                    m => Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("got invalid response {:?}", m))),
                },
            )
            .await?;
            let settings = request_single(
                &mut conn,
                move |tx| {
                    Task::Admin(
                        peer,
                        AdminRequest::SettingsGet {
                            scope: "com.actyx".parse().unwrap(),
                            no_defaults: false,
                        },
                        tx,
                    )
                },
                |resp| match resp {
                    AdminResponse::SettingsGetResponse(ls) => Ok(ls),
                    m => Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("got invalid response {:?}", m))),
                },
            )
            .await?;
            let offsets = request_single(
                &mut conn,
                move |tx| Task::Events(peer, EventsRequest::Offsets, tx),
                |m| match m {
                    EventsResponse::Offsets(o) => Ok(o),
                    x => Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("got invalid response {:?}", x))),
                },
            )
            .await?;

            let cbor = CborBuilder::new().encode_dict(|b| {
                b.with_key("nodeId", |b| b.write_bytes(node_info.node_id.as_ref(), []));
                b.with_key("displayName", |b| b.encode_str(node_info.display_name.clone()));
                b.with_key("totalEvents", |b| b.encode_u64(offsets.present.size()));
                b.with_key("timestamp", |b| b.encode_timestamp(now.into(), Precision::Nanos));
                b.with_key("actyxVersion", |b| b.encode_str(node_info.version.to_string()));
                b.with_key("axVersion", |b| b.encode_str(env!("AX_CLI_VERSION")));
                b.with_key("settings", |b| b.encode_str(settings.to_string()));
                b.with_key("connection", |b| {
                    b.encode_array(|b| {
                        b.encode_str(opts.console_opt.authority.original.as_str());
                        b.encode_str(opts.console_opt.authority.addrs.iter().map(|m| m.to_string()).join(","));
                    })
                });
                b.with_key("adminAddrs", |b| {
                    b.encode_array(|b| {
                        for addr in node_details.admin_addrs.iter() {
                            b.encode_str(addr);
                        }
                    })
                });
            });

            out.write_all(cbor.as_slice()).io("writing info block")?;

            diag.log("info block written")?;

            let (tx, mut events) = channel(20);
            conn.feed(Task::Events(
                peer,
                EventsRequest::Query(QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query,
                    order: Order::Asc,
                }),
                tx,
            ))
            .await?;

            let mut scratch = Vec::new();
            let mut count = 0u64;
            let mut max_size = cbor.as_slice().len();
            let mut last_printed = now;
            while let Some(ev) = events.next().await {
                let ev = ev?;
                match ev {
                    EventsResponse::Error { message } => diag.log(format!("AQL error: {}", message))?,
                    EventsResponse::Event(ev) => {
                        let cbor = CborBuilder::with_scratch_space(&mut scratch).encode_dict(|b| {
                            b.with_key("lamport", |b| b.encode_u64(ev.lamport.into()));
                            b.with_key("stream", |b| {
                                b.encode_array(|b| {
                                    b.write_bytes(ev.stream.node_id.as_ref(), []);
                                    b.encode_u64(ev.stream.stream_nr.into());
                                })
                            });
                            b.with_key("offset", |b| b.encode_u64(ev.offset.into()));
                            b.with_key("timestamp", |b| b.encode_u64(ev.timestamp.into()));
                            b.with_key("tags", |b| {
                                b.encode_array(|b| {
                                    for tag in ev.tags.iter() {
                                        b.encode_str(tag.as_ref());
                                    }
                                })
                            });
                            b.with_key("appId", |b| b.encode_str(&*ev.app_id));
                            b.with_key("payload", |b| b.write_trusting(ev.payload.as_slice()));
                        });
                        out.write_all(cbor.as_slice()).map_err(|e| {
                            ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error writing dump: {}", e))
                        })?;
                        count += 1;
                        max_size = max_size.max(cbor.as_slice().len());

                        let now = Local::now();
                        if now - last_printed > Duration::milliseconds(100) {
                            last_printed = now;
                            diag.status(format!("event {} ({})", count, DateTime::<Utc>::from(ev.timestamp)))?;
                        }
                    }
                    EventsResponse::Diagnostic(d) => diag.log(format!("diagnostic {:?}: {}", d.severity, d.message))?,
                    _ => {}
                }
            }
            diag.log(format!("{} events written (maximum size was {})", count, max_size))?;

            out.finish().io("finishing zstd")?;

            Ok(())
        }))
    }

    fn pretty(_result: Self::Output) -> String {
        String::new()
    }
}

struct WsWrite {
    sock: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl WsWrite {
    fn new(sock: WebSocket<MaybeTlsStream<TcpStream>>) -> Self {
        Self { sock }
    }
}

impl Write for WsWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sock
            .write_message(Message::Binary(buf.into()))
            .map(|_| buf.len())
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
