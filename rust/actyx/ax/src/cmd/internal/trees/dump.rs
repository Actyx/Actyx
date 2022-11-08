use actyx_sdk::Payload;
use banyan::{
    chacha20,
    store::{BranchCache, ReadOnlyStore},
    Forest, Secrets, TreeTypes,
};
use banyan_utils::{create_chacha_key, dump};
use futures::{prelude::*, stream, Stream};
use ipfs_sqlite_block_store::BlockStore;
use libipld::{
    cbor::DagCborCodec,
    codec::{Codec, Decode},
    json::DagJsonCodec,
};
use std::{convert::TryFrom, io::Cursor, path::PathBuf};
use structopt::StructOpt;
use trees::{
    axtrees::{AxKeySeq, AxTrees, Sha256Digest},
    AxTreeHeader,
};
use util::formats::{ActyxOSResult, ActyxOSResultExt};

use super::SqliteStore;
use crate::cmd::AxCliCommand;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
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
    /// Output dot. Sample usage: `ax _internal trees dump --block-store ..
    /// --dot --root .. | dot -Tpng > out.png`
    #[structopt(long)]
    dot: bool,
    /// Output all values, one per line
    #[structopt(long)]
    values: bool,
}

fn dump(opts: DumpTreeOpts) -> anyhow::Result<String> {
    match (opts.root, opts.block) {
        (Some(root), None) => {
            let secrets = {
                let index_key: chacha20::Key = opts.index_pass.map(create_chacha_key).unwrap_or_default();
                let value_key: chacha20::Key = opts.value_pass.map(create_chacha_key).unwrap_or_default();
                Secrets::new(index_key, value_key)
            };
            let bs = BlockStore::open(opts.block_store, Default::default())?;
            let ss = SqliteStore::new(bs)?;
            let forest = Forest::<AxTrees, _>::new(ss.clone(), BranchCache::new(1 << 20));
            let header: AxTreeHeader = DagCborCodec.decode(&ss.get(&root)?)?;
            let tree = forest.load_tree::<Payload>(secrets, header.root)?;
            if opts.dot {
                dump::graph(&forest, &tree, std::io::stdout())?;
            } else if opts.values {
                for maybe_pair in forest.iter_from(&tree) {
                    let (_, k, v) = maybe_pair?;
                    if opts.with_keys {
                        // This is a bit of an indirection ..
                        // DagCborEncode is only implemented for `AxKeySeq`
                        let seq: AxKeySeq = std::iter::once(k).collect();
                        let cbor = DagCborCodec.encode(&seq)?;
                        // Go to JSON via IPLD AST
                        let ipld = libipld::Ipld::decode(DagCborCodec, &mut Cursor::new(cbor))?;
                        let json = DagJsonCodec.encode(&ipld)?;
                        let key_seq: serde_json::Value = serde_json::from_slice(&json[..])?;
                        // And the pull out the original `AxKey` again:
                        println!(
                            "{}",
                            serde_json::json!({
                                "key":  {
                                    "time": key_seq["time"][0],
                                    "lamport": key_seq["lamport"][0],
                                    "tags": key_seq["tags"][0]
                                },
                                "value": v.json_value(),
                            })
                        );
                    } else {
                        println!("{}", v.cbor());
                    }
                }
            } else {
                forest.dump(&tree)?;
            }
        }
        (None, Some(block_hash)) => {
            let bs = BlockStore::open(opts.block_store, Default::default())?;
            let ss = SqliteStore::new(bs)?;
            let value_key: chacha20::Key = opts.value_pass.map(create_chacha_key).unwrap_or_default();
            let nonce = <&chacha20::XNonce>::try_from(AxTrees::NONCE).unwrap();
            dump::dump_cbor(ss, block_hash, &value_key, nonce, &mut std::io::stdout())?;
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
        Box::new(stream::once(
            async move { dump(opts).ax_err_ctx(util::formats::ActyxOSCode::ERR_INTERNAL_ERROR, "Dump failed") }.boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
