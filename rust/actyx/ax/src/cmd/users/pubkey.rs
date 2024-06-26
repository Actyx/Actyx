use crate::cmd::{load_identity, AxCliCommand};
use ax_core::{crypto::PublicKey, util::formats::ActyxOSResult};
use futures::{stream::once, FutureExt, Stream};

#[derive(clap::Parser, Clone, Debug)]
pub struct PubkeyOpts {
    /// Authentication identity (private key).
    /// Can be base64 encoded or a path to a file containing the key,
    /// defaults to `<OS_CONFIG_FOLDER>/key/users/id`.
    #[arg(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
    identity: Option<String>,
}

pub struct UsersPubkey;

impl AxCliCommand for UsersPubkey {
    type Opt = PubkeyOpts;
    type Output = PublicKey;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(once(
            async move { load_identity(&opts.identity).map(|private_key| private_key.to_public()) }.boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result.to_string()
    }
}
