use std::convert::TryInto;
use std::fmt::Write;

use crate::cmd::{AxCliCommand, ConsoleOpt};
use futures::{stream, FutureExt, Stream};
use prettytable::{cell, format, row, Table};
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, NodesInspectResponse};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// show node details
pub struct InspectOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

pub struct NodesInspect();
impl AxCliCommand for NodesInspect {
    type Opt = InspectOpts;
    type Output = NodesInspectResponse;
    fn run(mut opts: InspectOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            let response = opts
                .console_opt
                .authority
                .request(&opts.console_opt.identity.try_into()?, AdminRequest::NodesInspect)
                .await;
            match response {
                Ok(AdminResponse::NodesInspectResponse(resp)) => Ok(resp),
                Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
                Err(err) => Err(err),
            }
        }
        .boxed();
        Box::new(stream::once(fut))
    }

    fn pretty(result: Self::Output) -> String {
        let mut s = String::new();
        writeln!(&mut s, "PeerId: {}", result.peer_id).unwrap();
        writeln!(&mut s, "SwarmAddrs:").unwrap();
        for addr in &result.swarm_addrs {
            writeln!(&mut s, "    {}", addr).unwrap();
        }
        writeln!(&mut s, "AnnounceAddrs:").unwrap();
        for addr in &result.announce_addrs {
            writeln!(&mut s, "    {}", addr).unwrap();
        }
        writeln!(&mut s, "AdminAddrs:").unwrap();
        for addr in &result.admin_addrs {
            writeln!(&mut s, "    {}", addr).unwrap();
        }
        writeln!(&mut s, "Connections:").unwrap();
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.set_titles(row!["PEERID", "ADDRESS"]);
        for row in &result.connections {
            table.add_row(row![row.peer_id, row.addr,]);
        }
        writeln!(&mut s, "{}", table).unwrap();
        writeln!(&mut s, "KnownPeers:").unwrap();
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.set_titles(row!["PEERID", "ADDRESS"]);
        for peer in &result.known_peers {
            for (i, addr) in peer.addrs.iter().enumerate() {
                if i == 0 {
                    table.add_row(row![peer.peer_id, addr,]);
                } else {
                    table.add_row(row!["", addr,]);
                }
            }
        }
        writeln!(&mut s, "{}", table).unwrap();
        s
    }
}
