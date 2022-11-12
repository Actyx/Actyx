use std::{convert::TryInto, time::Duration};

use crate::{
    cmd::{consts::TABLE_FORMAT, Authority, AxCliCommand, KeyPathWrapper},
    node_connection::{connect, mk_swarm, request_single, Task},
    private_key::AxPrivateKey,
};
use futures::{channel::mpsc, future::join_all, stream, Stream};
use prettytable::{cell, row, Table};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, NodesLsResponse};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// show node overview
pub struct LsOpts {
    #[structopt(name = "NODE", required = true)]
    /// the IP address or <host>:<admin port> of the nodes to list.
    authority: Vec<Authority>,
    #[structopt(short, long)]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
    #[structopt(short, long, default_value = "5")]
    /// maximal wait time (in seconds, max. 255) for establishing a connection to the node
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
    table.set_format(*TABLE_FORMAT);
    table.set_titles(row!["NODE ID", "DISPLAY NAME", "HOST", "STARTED", "VERSION"]);

    for row in output {
        match row {
            Output::Reachable(json) => {
                table.add_row(row![
                    json.resp.node_id,
                    json.resp.display_name,
                    json.host,
                    json.resp.started_iso,
                    json.resp.version
                ]);
            }
            Output::Unreachable { host } => {
                table.add_row(row!["Actyx was unreachable on host", "", host]);
            }
            Output::Unauthorized { host } => {
                table.add_row(row!["Unauthorized on host", "", host]);
            }
            Output::Error { host, error } => {
                table.add_row(row![format!("{}", error), "", host]);
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
