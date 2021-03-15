use actyxos_sdk::Payload;
use banyan::{
    forest::{BranchCache, Config, CryptoConfig, Forest},
    salsa20,
};
use banyan_utils::{create_salsa_key, dump};
use futures::{prelude::*, stream, Stream};
use ipfs_sqlite_block_store::BlockStore;
use std::{path::PathBuf, sync::Arc};
use structopt::StructOpt;
use trees::axtrees::{AxTrees, Sha256Digest};
use util::formats::{ActyxOSResult, ActyxOSResultExt};

use super::SqliteStore;
use crate::cmd::AxCliCommand;

#[derive(StructOpt, Debug)]
pub struct DumpTreeOpts {
    #[structopt(long)]
    /// Path to a sqlite blockstore (read-only access!)
    block_store: PathBuf,
    #[structopt(long)]
    /// Index password to use
    index_pass: Option<String>,
    #[structopt(long)]
    /// Value password to use
    value_pass: Option<String>,
    #[structopt(long)]
    /// Dump a tree with a given root. Per default, only the extracted values are
    /// being printed. Set the --with-keys flag to emit those as well.
    root: Option<Sha256Digest>,
    #[structopt(long)]
    /// Dump the raw block data as json
    block: Option<Sha256Digest>,
    #[structopt(long)]
    /// When dumping all values from a tree, also include the keys.
    with_keys: bool,
}

fn dump(opts: DumpTreeOpts) -> anyhow::Result<String> {
    match (opts.root, opts.block) {
        (Some(root), None) => {
            let crypto_config = {
                let index_key: salsa20::Key = opts.index_pass.map(create_salsa_key).unwrap_or_default();
                let value_key: salsa20::Key = opts.value_pass.map(create_salsa_key).unwrap_or_default();
                CryptoConfig { index_key, value_key }
            };
            let bs = BlockStore::open(opts.block_store, Default::default())?;
            let ss = SqliteStore::new(bs)?;
            let forest = Forest::<AxTrees, Payload, _>::new(ss, BranchCache::new(1000), crypto_config, Config::debug());
            let tree = forest.load_tree(root)?;

            for maybe_pair in forest.iter_from(&tree, 0) {
                let (_, k, v) = maybe_pair?;
                if opts.with_keys {
                    // TODO: more structured output?
                    println!("{:?} --> {}", k, v.json_string());
                } else {
                    println!("{}", v.json_string());
                }
            }
        }
        (None, Some(block_hash)) => {
            let bs = BlockStore::open(opts.block_store, Default::default())?;
            let ss = SqliteStore::new(bs)?;
            let value_key: salsa20::Key = opts.value_pass.map(create_salsa_key).unwrap_or_default();
            dump::dump_json(Arc::new(ss), block_hash, value_key, &mut std::io::stdout())?;
        }
        _ => anyhow::bail!("Provide either root or block hash"),
    }

    Ok("".into())
}

pub struct DumpTree;
impl AxCliCommand for DumpTree {
    type Opt = DumpTreeOpts;
    type Output = String;
    fn run(opts: DumpTreeOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(stream::once(async move { dump(opts).ax_internal() }.boxed()))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
