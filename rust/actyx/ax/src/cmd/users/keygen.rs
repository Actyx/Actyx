use crate::private_key::DEFAULT_PRIVATE_KEY_FILE_NAME;
use crate::{cmd::AxCliCommand, private_key::AxPrivateKey};
use futures::{stream, Stream};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::io::AsyncBufReadExt;
use util::{
    ax_bail,
    formats::{ax_err, ActyxOSResult},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    private_key_path: PathBuf,
    public_key_path: PathBuf,
    public_key: String,
}

pub struct UsersKeygen();
impl AxCliCommand for UsersKeygen {
    type Opt = KeygenOpts;
    type Output = Output;
    fn run(mut opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(async move {
            let key = AxPrivateKey::generate();
            eprintln!("Generating public/private key pair ..");
            let store_to = if let Some(p) = opts.output.take() {
                p
            } else {
                let default =
                    AxPrivateKey::get_and_create_default_user_identity_dir()?.join(DEFAULT_PRIVATE_KEY_FILE_NAME);
                eprint!("Enter path in which to save the key ({}): ", default.display());
                let io = tokio::io::stdin();
                let mut reader = tokio::io::BufReader::new(io);
                let mut buf = String::new();
                reader.read_line(&mut buf).await?;
                // pop '\n'
                buf.pop();
                if buf.is_empty() {
                    default
                } else {
                    buf.into()
                }
            };
            if store_to.exists() {
                ax_bail!(
                    util::formats::ActyxOSCode::ERR_FILE_EXISTS,
                    "File {} already exits in the specified path. Specify a different file name or path.",
                    store_to.display()
                );
            }
            let (private_key_path, public_key_path) = key.to_file(&store_to)?;
            let public_key = key.to_string();
            Ok(Output {
                private_key_path,
                public_key,
                public_key_path,
            })
        });
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        format!(
            "Your private key has been saved at {}\nYour public key has been saved at {}\nThe key's fingerprint is: {}",
            result.private_key_path.display(),
            result.public_key_path.display(),
            result.public_key
        )
    }
}
#[derive(StructOpt, Debug)]
pub struct KeygenOpts {
    #[structopt(short, long)]
    output: Option<PathBuf>,
}
