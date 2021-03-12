use crate::cmd::formats::Result;
use crate::cmd::AxCliCommand;
use futures::{stream, Stream, TryFutureExt};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSResult, ActyxOSResultExt};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    swarm_key: String,
    output_path: Option<String>,
}
pub struct SwarmsKeygen();
impl AxCliCommand for SwarmsKeygen {
    type Opt = KeygenOpts;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        if let Some(path) = result.output_path {
            format!("Key written to {}", path)
        } else {
            result.swarm_key
        }
    }
}
#[derive(StructOpt, Debug)]
pub struct KeygenOpts {
    #[structopt(short, long, parse(from_os_str))]
    /// Create file <output> and write the generated key to it.
    pub(crate) output: Option<PathBuf>,
}

/// https://github.com/Kubuxu/go-ipfs-swarm-key-gen/blob/master/ipfs-swarm-key-gen/main.go
pub fn generate_key() -> String {
    let key_length = 32;
    let key: Vec<u8> = rand::thread_rng().sample_iter(&Alphanumeric).take(key_length).collect();
    let key = format!("/key/swarm/psk/1.0.0/\n/base16/\n{}", hex::encode(key));
    base64::encode(&key.as_bytes())
}

pub fn store_key(key: String, mut path: PathBuf) -> Result<()> {
    if path.is_dir() {
        path.push("actyx-swarm.key");
    }
    if path.exists() {
        return Err(ActyxOSCode::ERR_INVALID_INPUT.with_message(format!(
            "Cannot write swarm key to file since file '{}' already exists.",
            path.display()
        )));
    }
    std::fs::write(&path, key).ax_err_ctx(ActyxOSCode::ERR_IO, format!("Error writing to {}", path.display()))?;
    Ok(())
}

pub async fn run(opt: KeygenOpts) -> Result<Output> {
    let key = generate_key();
    if let Some(path) = opt.output.clone() {
        store_key(key.clone(), path)?;
    }
    Ok(Output {
        swarm_key: key,
        output_path: opt.output.map(|p| p.display().to_string()),
    })
}

#[cfg(test)]
mod test {
    use crate::cmd::formats::Result;
    use crate::cmd::swarms::keygen::{generate_key, run, store_key, KeygenOpts};

    #[tokio::test]
    pub async fn should_store_swarm_key() -> Result<()> {
        let key = generate_key();
        assert_eq!(128, key.len());

        let tempdir = tempfile::tempdir().unwrap();
        let mut p = tempdir.path().to_owned();
        store_key(key, p.clone())?;

        // It should add the filename
        p.push("actyx-swarm.key");
        assert!(p.as_path().exists());
        let key = std::fs::read_to_string(&p).unwrap();
        let key: String = String::from_utf8(base64::decode(&key).unwrap()).unwrap();
        let key: Vec<&str> = key.lines().collect();

        assert_eq!(3, key.len());
        assert_eq!(key[0], "/key/swarm/psk/1.0.0/");
        assert_eq!(key[1], "/base16/");
        assert_eq!(key[2].len(), 64);

        let res = run(KeygenOpts { output: Some(p) }).await;
        // File already exists
        res.unwrap_err();

        Ok(())
    }
}
