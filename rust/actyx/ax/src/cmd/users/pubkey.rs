use std::path::PathBuf;

use crate::cmd::AxCliCommand;
use ax_core::{crypto::PublicKey, private_key::AxPrivateKey, util::formats::ActyxOSResult};
use futures::{stream::once, FutureExt, Stream};

#[derive(clap::Parser, Clone, Debug)]
pub struct PubkeyOpts {
    /// File from which the identity (private key) for authentication is read.
    #[arg(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
    identity: Option<PathBuf>,
}

pub struct UsersPubkey;

impl AxCliCommand for UsersPubkey {
    type Opt = PubkeyOpts;
    type Output = PublicKey;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(once(
            async move {
                opts.identity
                    .map_or_else(AxPrivateKey::load_from_default_path, AxPrivateKey::from_file)
                    .map(|key| key.to_public())
            }
            .boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result.to_string()
    }
}
