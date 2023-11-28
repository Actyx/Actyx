use crate::cmd::{consts::TABLE_FORMAT, Authority, AxCliCommand, KeyPathWrapper};
use ax_core::util::formats::{
    ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, TopicDeleteResponse,
};
use ax_core::{
    node_connection::{connect, mk_swarm, request_single, Task},
    private_key::AxPrivateKey,
};
use futures::{channel::mpsc, future::join_all, stream};
use prettytable::{cell, row, Table};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use structopt::StructOpt;

pub struct TopicsDelete;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "connection")]
pub enum DeleteOutput {
    Reachable {
        host: String,
        response: TopicDeleteResponse,
    },
    Unreachable {
        host: String,
    },
    Unauthorized {
        host: String,
    },
    Error {
        host: String,
        error: ActyxOSError,
    },
}

// This code is mostly duplicated from `ax/src/cmd/nodes/ls.rs`
async fn request(timeout: u8, mut conn: mpsc::Sender<Task>, authority: Authority, topic_name: String) -> DeleteOutput {
    let host = authority.original.clone();
    let response = tokio::time::timeout(Duration::from_secs(timeout.into()), async move {
        let peer = connect(&mut conn, authority).await?;
        request_single(
            &mut conn,
            move |tx| Task::Admin(peer, AdminRequest::TopicDelete { name: topic_name }, tx),
            Ok,
        )
        .await
    })
    .await;
    if let Ok(response) = response {
        match response {
            Ok(AdminResponse::TopicDeleteResponse(response)) => DeleteOutput::Reachable { host, response },
            Ok(response) => DeleteOutput::Error {
                host,
                error: ActyxOSError::internal(format!("Unexpected response from node: {:?}", response)),
            },
            Err(error) if error.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => DeleteOutput::Unreachable { host },
            Err(error) if error.code() == ActyxOSCode::ERR_UNAUTHORIZED => DeleteOutput::Unauthorized { host },
            Err(error) => DeleteOutput::Error { host, error },
        }
    } else {
        // The difference between this unreachable and the previous lies on the timeout
        // here `ax` is "giving up" and on the previous, the node is actually unreachable
        DeleteOutput::Error {
            host,
            error: ActyxOSError::new(ax_core::util::formats::ActyxOSCode::ERR_NODE_UNREACHABLE, "timeout"),
        }
    }
}

async fn delete_run(opts: DeleteOpts) -> ActyxOSResult<Vec<DeleteOutput>> {
    // Get the auth and timeout parameters
    let identity: AxPrivateKey = (&opts.identity).try_into()?;
    let timeout = opts.timeout;
    // Get a communication channel to the swarm
    let (task, channel) = mk_swarm(identity).await?;
    tokio::spawn(task);
    // Send the ls command to all nodes in authority and return the results
    Ok(join_all(
        opts.authority
            .into_iter()
            .map(|a| request(timeout, channel.clone(), a, opts.topic.clone()))
            .collect::<Vec<_>>(),
    )
    .await)
}

impl AxCliCommand for TopicsDelete {
    type Opt = DeleteOpts;

    type Output = Vec<DeleteOutput>;

    fn run(
        opts: Self::Opt,
    ) -> Box<dyn futures::Stream<Item = ax_core::util::formats::ActyxOSResult<Self::Output>> + Unpin> {
        let requests = Box::pin(delete_run(opts));
        Box::new(stream::once(requests))
    }

    fn pretty(result: Self::Output) -> String {
        let mut table = Table::new();
        table.set_format(*TABLE_FORMAT);
        table.set_titles(row!["NODE ID", "HOST", "DELETED"]);
        for output in result {
            match output {
                DeleteOutput::Reachable { host, response } => {
                    table.add_row(row![response.node_id, host, if response.deleted { "Y" } else { "N" }]);
                }
                DeleteOutput::Unreachable { host } => {
                    table.add_row(row!["Actyx was unreachable on host", host]);
                }
                DeleteOutput::Unauthorized { host } => {
                    table.add_row(row!["Unauthorized on host", host]);
                }
                DeleteOutput::Error { host, error } => {
                    table.add_row(row![format!("Received error \"{}\" from host", error), host]);
                }
            }
        }
        table.to_string()
    }
}

/// Delete selected topic
#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
pub struct DeleteOpts {
    /// The topic to delete.
    #[structopt(required = true)]
    topic: String,
    /// The IP addresses or <host>:<admin port> of the target nodes.
    #[structopt(name = "NODE", required = true)]
    authority: Vec<Authority>,
    /// The private key file to use for authentication.
    #[structopt(short, long)]
    identity: Option<KeyPathWrapper>,
    /// Timeout time for the operation (in seconds, with a maximum of 255).
    #[structopt(short, long, default_value = "5")]
    timeout: u8,
}
