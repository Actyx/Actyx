use ax_core::{
    swarm::StreamAlias,
    util::formats::{ActyxOSResult, ActyxOSResultExt},
};
use ax_sdk::StreamId;
use futures::{prelude::*, stream, Stream};
use ipfs_sqlite_block_store::BlockStore;
use libipld::{Cid, DefaultParams};
use std::{convert::TryFrom, path::PathBuf};

use crate::cmd::AxCliCommand;

#[derive(clap::ValueEnum, Debug, Clone)]
enum List {
    /// List all aliases that resolve to stream ids, and their respective root
    /// hashes
    Aliases,
    /// List all cids in the blockstore
    Cids,
    /// List blocks cids in the blockstore
    Blocks,
}

#[derive(clap::Parser, Clone, Debug)]
pub struct ExploreTreeOpts {
    /// Path to a sqlite blockstore (read-only access!)
    #[arg(long)]
    block_store: PathBuf,
    command: List,
}

fn run(opts: ExploreTreeOpts) -> anyhow::Result<()> {
    let mut bs = BlockStore::<DefaultParams>::open(opts.block_store.clone(), Default::default())?;
    match opts.command {
        List::Aliases => {
            let aliases: Vec<(Vec<u8>, Cid)> = bs.aliases()?;
            for (bytes, cid) in aliases {
                let x = StreamAlias::try_from(&bytes[..])?;
                if let Ok(x) = StreamId::try_from(x) {
                    println!("{} --> {}", x, cid);
                }
            }
        }
        List::Cids => {
            let cids: Vec<Cid> = bs.get_known_cids()?;
            for cid in cids {
                println!("{}", cid);
            }
        }
        List::Blocks => {
            let cids: Vec<Cid> = bs.get_block_cids()?;
            for cid in cids {
                println!("{}", cid);
            }
        }
    }

    Ok(())
}

pub struct ExploreTree;
impl AxCliCommand for ExploreTree {
    type Opt = ExploreTreeOpts;
    type Output = ();
    fn run(opts: ExploreTreeOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(stream::once(
            async move { run(opts).ax_err_ctx(ax_core::util::formats::ActyxOSCode::ERR_INTERNAL_ERROR, "run failed") }
                .boxed(),
        ))
    }

    fn pretty(_: Self::Output) -> String {
        "".into()
    }
}
