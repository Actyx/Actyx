use crate::{
    cmd::{AxCliCommand, KeyPathWrapper},
    private_key::AxPrivateKey,
};
use crypto::PublicKey;
use futures::{stream::once, FutureExt, Stream};
use structopt::StructOpt;
use util::formats::ActyxOSResult;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub struct PubkeyOpts {
    #[structopt(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
}

pub struct UsersPubkey;

impl AxCliCommand for UsersPubkey {
    type Opt = PubkeyOpts;
    type Output = PublicKey;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(once(
            async move { AxPrivateKey::try_from(&opts.identity).map(|p| p.to_public()) }.boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result.to_string()
    }
}
