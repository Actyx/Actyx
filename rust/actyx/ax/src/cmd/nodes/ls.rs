use crate::cmd::{Authority, AxCliCommand};
use ax_core::{
    node_connection::{connect, mk_swarm, request_single, Task},
    private_key::{AxPrivateKey, KeyPathWrapper},
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, NodesLsResponse},
};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Table};
use futures::{channel::mpsc, future::join_all, stream, Stream};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, time::Duration};

#[derive(clap::Parser, Clone, Debug)]
/// show node overview
pub struct LsOpts {
    /// the IP address or `<host>:<admin port>` of the nodes to list.
    #[arg(name = "NODE", required = true)]
    authority: Vec<Authority>,
    /// File from which the identity (private key) for authentication is read.
    #[arg(short, long)]
    identity: Option<KeyPathWrapper>,
    /// maximal wait time (in seconds, max. 255) for establishing a connection to the node
    #[arg(short, long, default_value = "5")]
    timeout: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonFormat {
    host: String,
    #[serde(flatten)]
    resp: NodesLsResponse,
}
impl JsonFormat {
    fn from_resp(host: String, resp: NodesLsResponse) -> JsonFormat {
        JsonFormat { host, resp }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "connection")]
#[serde(rename_all = "camelCase")]
pub enum Output {
    Reachable(JsonFormat),
    Unreachable { host: String },
    Unauthorized { host: String },
    Error { host: String, error: ActyxOSError },
}
fn format_output(output: Vec<Output>) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_header(["NODE ID", "DISPLAY NAME", "HOST", "STARTED", "VERSION"]);

    for row in output {
        match row {
            Output::Reachable(json) => {
                table.add_row([
                    Cell::new(json.resp.node_id),
                    Cell::new(json.resp.display_name),
                    Cell::new(json.host),
                    Cell::new(json.resp.started_iso),
                    Cell::new(json.resp.version),
                ]);
            }
            Output::Unreachable { host } => {
                table.add_row([Cell::new("AX was unreachable on host"), Cell::new(""), Cell::new(host)]);
            }
            Output::Unauthorized { host } => {
                table.add_row([Cell::new("Unauthorized on host"), Cell::new(""), Cell::new(host)]);
            }
            Output::Error { host, error } => {
                table.add_row([Cell::new(error), Cell::new(""), Cell::new(host)]);
            }
        }
    }
    table.to_string()
}

async fn request(timeout: u8, mut conn: mpsc::Sender<Task>, authority: Authority) -> Output {
    let host = authority.original.clone();
    let response = tokio::time::timeout(Duration::from_secs(timeout.into()), async move {
        let peer = connect(&mut conn, authority).await?;
        request_single(&mut conn, move |tx| Task::Admin(peer, AdminRequest::NodesLs, tx), Ok).await
    })
    .await;
    match response {
        Ok(Ok(AdminResponse::NodesLsResponse(resp))) => Output::Reachable(JsonFormat::from_resp(host, resp)),
        Ok(Err(err)) if err.code() == ActyxOSCode::ERR_UNAUTHORIZED => Output::Unauthorized { host },
        Ok(Err(err)) if err.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => Output::Unreachable { host },
        Ok(Ok(e)) => Output::Error {
            host,
            error: ActyxOSError::internal(format!("Unexpected response from node: {:?}", e)),
        },
        Ok(Err(e)) => Output::Error { host, error: e },
        Err(_) => Output::Error {
            host,
            error: ActyxOSError::new(ActyxOSCode::ERR_NODE_UNREACHABLE, "timeout"),
        },
    }
}

async fn run(opts: LsOpts) -> ActyxOSResult<Vec<Output>> {
    let identity: AxPrivateKey = (&opts.identity).try_into()?;
    let timeout = opts.timeout;
    let (task, channel) = mk_swarm(identity).await?;
    tokio::spawn(task);
    Ok(join_all(
        opts.authority
            .into_iter()
            .map(|a| request(timeout, channel.clone(), a))
            .collect::<Vec<_>>(),
    )
    .await)
}

pub struct NodesLs();
impl AxCliCommand for NodesLs {
    type Opt = LsOpts;
    type Output = Vec<Output>;
    fn run(opts: LsOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let requests = Box::pin(run(opts));
        Box::new(stream::once(requests))
    }

    fn pretty(result: Self::Output) -> String {
        // In order to properly render the table, all results have to be
        // present.
        format_output(result)
    }
}
