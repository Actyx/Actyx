use crate::cmd::AxCliCommand;
use ax_core::{
    private_key::generate_key,
    util::formats::{ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;

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
#[structopt(version = ax_core::util::version::VERSION.as_str())]
/// generate swarm key
pub struct KeygenOpts {
    /// Create file <output> and write the generated key to it.
    #[structopt(short, long, parse(from_os_str))]
    pub(crate) output: Option<PathBuf>,
}

pub fn store_key(key: String, mut path: PathBuf) -> ActyxOSResult<()> {
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

pub async fn run(opt: KeygenOpts) -> ActyxOSResult<Output> {
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
    use ax_core::{private_key::generate_key, util::formats::ActyxOSResult};

    use crate::cmd::swarms::keygen::{run, store_key, KeygenOpts};

    #[tokio::test]
    pub async fn should_store_swarm_key() -> ActyxOSResult<()> {
        let key = generate_key();
        assert_eq!(44, key.len());

        let tempdir = tempfile::tempdir().unwrap();
        let mut p = tempdir.path().to_owned();
        store_key(key, p.clone())?;

        // It should add the filename
        p.push("actyx-swarm.key");
        assert!(p.as_path().exists());
        let key = std::fs::read_to_string(&p).unwrap();
        let key = base64::decode(&key).unwrap();
        assert_eq!(key.len(), 32);

        let res = run(KeygenOpts { output: Some(p) }).await;
        // File already exists
        res.unwrap_err();

        Ok(())
    }
}
