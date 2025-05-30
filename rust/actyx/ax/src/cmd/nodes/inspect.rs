use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_single, Task},
    util::{
        formats::{ActyxOSCode, ActyxOSResult, AdminRequest, AdminResponse, NodesInspectResponse},
        version::NodeVersion,
    },
};
use ax_sdk::types::NodeId;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, CellAlignment, Table};
use futures::{stream, FutureExt, Stream};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(clap::Parser, Clone, Debug)]
/// show node details
pub struct InspectOpts {
    #[command(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    node_id: Option<NodeId>,
    node_version: Option<NodeVersion>,
    #[serde(flatten)]
    inspect: NodesInspectResponse,
}

pub struct NodesInspect();
impl AxCliCommand for NodesInspect {
    type Opt = InspectOpts;
    type Output = Output;
    fn run(opts: InspectOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let (node_id, node_version) = request_single(&mut conn, move |tx| Task::NodeId(peer, tx), Ok)
                .await
                .ok()
                .map(|p| (Some(p.0), Some(p.1)))
                .unwrap_or((None, None));
            let inspect = request_single(
                &mut conn,
                move |tx| Task::Admin(peer, AdminRequest::NodesInspect, tx),
                |m| match m {
                    AdminResponse::NodesInspectResponse(r) => Ok(r),
                    x => Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("invalid response: {:?}", x))),
                },
            )
            .await?;
            Ok(Output {
                node_id,
                node_version,
                inspect,
            })
        }
        .boxed();
        Box::new(stream::once(fut))
    }

    fn pretty(result: Self::Output) -> String {
        let mut s = String::new();
        let Output {
            node_id,
            node_version,
            inspect: result,
        } = result;
        writeln!(&mut s, "PeerId: {}", result.peer_id).unwrap();
        if let Some(node_id) = node_id {
            writeln!(&mut s, "NodeId: {}", node_id).unwrap()
        }
        if let Some(node_version) = node_version {
            writeln!(&mut s, "Node version: {}", node_version).unwrap()
        }

        writeln!(&mut s, "SwarmAddrs:").unwrap();
        for addr in &result.swarm_addrs {
            writeln!(&mut s, "    {}", addr).unwrap();
        }

        writeln!(&mut s, "AnnounceAddrs:").unwrap();
        if result.announce_addrs.is_empty() {
            writeln!(&mut s, "  none").unwrap();
        } else {
            for addr in &result.announce_addrs {
                writeln!(&mut s, "    {}", addr).unwrap();
            }
        }

        writeln!(&mut s, "AdminAddrs:").unwrap();
        for addr in &result.admin_addrs {
            writeln!(&mut s, "    {}", addr).unwrap();
        }

        writeln!(&mut s, "Connections:").unwrap();
        if result.connections.is_empty() {
            writeln!(&mut s, "  none").unwrap();
        } else {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .set_header(["PEERID", "ADDRESS", "DIRECTION", "SINCE"]);
            for row in &result.connections {
                let direction = if row.since.is_empty() {
                    ""
                } else if row.outbound {
                    "outbound"
                } else {
                    "inbound"
                };
                table.add_row([
                    Cell::new(&row.peer_id),
                    Cell::new(&row.addr),
                    Cell::new(direction),
                    Cell::new(&row.since),
                ]);
            }
            writeln!(&mut s, "{}", table).unwrap();
        }

        let mut failures = Vec::new();
        let mut ping = Table::new();
        ping.load_preset(UTF8_FULL_CONDENSED).set_header([
            "PEERID",
            "CURRENT",
            "AVG_3",
            "AVG_10",
            "FAILURES",
            "FAILURE_RATE",
        ]);

        writeln!(&mut s, "KnownPeers (more details with --json):").unwrap();
        if result.known_peers.is_empty() {
            writeln!(&mut s, "  none").unwrap();
        } else {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .set_header(["PEERID", "NAME", "ADDRESS", "SOURCE", "SINCE"]);
            for peer in &result.known_peers {
                for (i, addr) in peer.addrs.iter().enumerate() {
                    let p = if i == 0 { &*peer.peer_id } else { "" };
                    let n = peer
                        .info
                        .agent_version
                        .as_deref()
                        .filter(|_| i == 0)
                        .unwrap_or_default();
                    let source = peer.addr_source.get(i).map(String::as_str).unwrap_or_default();
                    let since = peer.addr_since.get(i).map(String::as_str).unwrap_or_default();
                    table.add_row([p, n, addr, source, since]);
                }

                for f in &peer.failures {
                    failures.push((
                        f.time.clone(),
                        f.addr.clone(),
                        peer.peer_id.to_string(),
                        f.display.clone(),
                    ));
                }

                if let Some(rtt) = &peer.ping_stats {
                    ping.add_row([
                        Cell::new(&peer.peer_id).set_alignment(CellAlignment::Right),
                        Cell::new(format_micros(rtt.current)),
                        Cell::new(format_micros(rtt.decay_3)),
                        Cell::new(format_micros(rtt.decay_10)),
                        Cell::new(rtt.failures),
                        Cell::new(format!("{:.4}%", rtt.failure_rate as f64 / 10_000.0)),
                    ]);
                }
            }
            writeln!(&mut s, "{}", table).unwrap();
        }

        writeln!(&mut s, "Failures (more details with --json):").unwrap();
        if failures.is_empty() {
            writeln!(&mut s, "  none").unwrap();
        } else {
            failures.sort();
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .set_header(["TIME", "ADDRESS", "PEERID", "MESSAGE"]);
            for f in failures {
                table.add_row([f.0, f.1, f.2, f.3]);
            }
            writeln!(&mut s, "{}", table).unwrap();
        }

        writeln!(&mut s, "Ping statistics:").unwrap();
        if ping.is_empty() {
            writeln!(&mut s, "  none").unwrap();
        } else {
            writeln!(&mut s, "{}", ping).unwrap();
        }

        s
    }
}

fn format_micros(n: u32) -> String {
    if n >= 10_000 {
        format!("{}ms", (n + 500) / 1000)
    } else {
        format!("{}µs", n)
    }
}
