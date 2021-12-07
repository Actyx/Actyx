use super::dump::Diag;
use crate::cmd::{AxCliCommand, ConsoleOpt};
use cbor_data::Cbor;
use futures::Stream;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};
use structopt::StructOpt;
use util::{
    formats::{
        banyan_protocol::{decode_dump_header, BanyanRequest, BanyanResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult,
    },
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// dump events described by an AQL query into a file
pub struct RestoreOpts {
    #[structopt(name = "INPUT", required = true)]
    /// file to read the dump from
    input: PathBuf,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    #[structopt(long, short)]
    quiet: bool,
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
            let mut diag = Diag::new(opts.quiet);

            let mut input: Box<dyn Read> = if opts.input.to_str() == Some("-") {
                Box::new(std::io::stdin())
            } else {
                Box::new(File::open(opts.input).io("opening dump")?)
            };

            let mut buf = Vec::new();
            buf.resize(100_000, 0u8);
            let mut pos = 0;
            loop {
                let len = input.read(&mut buf.as_mut_slice()[pos..]).io("reading dump")?;
                pos += len;
                if len == 0 || pos == buf.len() {
                    break;
                }
            }

            let mut decoder = zstd::stream::write::Decoder::new(Vec::new()).io("starting decoder")?;
            decoder.write_all(&buf.as_slice()[..pos]).io("decoding header")?;
            decoder.flush().io("flushing header")?;

            let (cbor, _rest) = Cbor::checked_prefix(decoder.get_ref().as_slice()).map_err(|e| {
                ActyxOSError::new(
                    ActyxOSCode::ERR_INVALID_INPUT,
                    format!("cannot read dump header: {}", e),
                )
            })?;
            let (node_id, topic, timestamp) = decode_dump_header(cbor)
                .ok_or_else(|| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, "cannot read dump header"))?;
            // keep the bytes in the buffer because the Actyx node will need to read the header as well

            diag.log(format!("sending dump from node {} topic `{}`", node_id, topic))?;
            let topic = format!("dump-{}", timestamp.to_rfc3339());
            diag.log(format!("uploading to topic `{}`", topic))?;

            let mut conn = opts.console_opt.connect().await?;

            conn.request_banyan(BanyanRequest::MakeFreshTopic(topic.clone()))
                .await?
                .br()?;
            let mut count = 0;
            loop {
                conn.request_banyan(BanyanRequest::AppendEvents(topic.clone(), buf[..pos].into()))
                    .await?
                    .br()?;
                count += pos;
                diag.status(format!("{} bytes uploaded", count))?;
                pos = input.read(buf.as_mut_slice()).io("reading dump")?;
                if pos == 0 {
                    break;
                }
            }
            diag.log(format!("in total {} bytes uploaded", count))?;
            conn.request_banyan(BanyanRequest::Finalise(topic.clone()))
                .await?
                .br()?;
            diag.log(format!("topic switched to `{}`", topic))?;

            Ok(())
        }))
    }

    fn pretty(_result: Self::Output) -> String {
        String::new()
    }
}
