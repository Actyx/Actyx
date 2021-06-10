use std::{convert::TryInto, time::Duration};

use crate::{
    cmd::{formats::Result, AxCliCommand, KeyPathWrapper, NodeConnection},
    private_key::AxPrivateKey,
};
use futures::{future::try_join_all, stream, Stream};
use prettytable::{cell, format, row, Table};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, NodesLsResponse};

#[derive(StructOpt, Debug)]
pub struct LsOpts {
    #[structopt(name = "NODE", required = true)]
    /// Node ID or, if using `--local`, the IP address of the node to perform the
    /// operation on.
    authority: Vec<NodeConnection>,
    #[structopt(short, long)]
    /// Process over local network
    local: bool,
    #[structopt(short, long)]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
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
}
fn format_output(output: Vec<Output>) -> String {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
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
                table.add_row(row![format!("Actyx was unreachable on host: {}", host)]);
            }
            Output::Unauthorized { host } => {
                table.add_row(row![format!("Unauthorized: {}", host)]);
            }
        }
    }
    table.to_string()
}

async fn request(identity: AxPrivateKey, mut connection: NodeConnection) -> Result<Output> {
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        connection.request(&identity, AdminRequest::NodesLs),
    )
    .await;
    match response {
        Ok(Ok(AdminResponse::NodesLsResponse(resp))) => {
            Ok(Output::Reachable(JsonFormat::from_resp(connection.original, resp)))
        }
        Ok(Err(err)) if err.code() == ActyxOSCode::ERR_UNAUTHORIZED => Ok(Output::Unauthorized {
            host: connection.original,
        }),
        Ok(Err(err)) if err.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => Ok(Output::Unreachable {
            host: connection.original,
        }),
        Ok(Ok(e)) => Err(ActyxOSError::internal(format!(
            "Unexpected response from node: {:?}",
            e
        ))),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(Output::Unreachable {
            host: connection.original,
        }),
    }
}

async fn run(opts: LsOpts) -> ActyxOSResult<Vec<Output>> {
    if !opts.local {
        return Err(ActyxOSCode::ERR_INVALID_INPUT
            .with_message("This version of ax currently only supports local interactions using the --local flag."));
    }

    let identity: AxPrivateKey = opts.identity.try_into()?;
    try_join_all(
        opts.authority
            .into_iter()
            .map(|a| request(identity.clone(), a))
            .collect::<Vec<_>>(),
    )
    .await
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
