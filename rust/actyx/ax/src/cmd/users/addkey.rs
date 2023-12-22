use crate::cmd::{load_identity, AxCliCommand};
use ax_core::{
    settings::{Database, Repository, Scope, DB_FILENAME},
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult},
};
use futures::{stream, Stream};
use std::{path::PathBuf, str::FromStr};

fn lock_working_dir(working_dir: impl AsRef<std::path::Path>) -> ActyxOSResult<fslock::LockFile> {
    let path = working_dir.as_ref().join("lockfile");
    println!("locking {}", path.display());
    let mut lf = fslock::LockFile::open(&path)
        .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error opening lockfile: {}", e)))?;
    if !lf
        .try_lock()
        .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error locking lockfile: {}", e)))?
    {
        return Err(ActyxOSError::new(
            ActyxOSCode::ERR_FILE_EXISTS,
            "AX directory is in use, please stop AX first!".to_owned(),
        ));
    }
    Ok(lf)
}

pub struct UsersAddKey();
impl AxCliCommand for UsersAddKey {
    type Opt = AddKeyOpts;
    type Output = ();
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(async move {
            let privkey = load_identity(&opts.identity)?;
            let pubkey = privkey.to_public();

            // check that the path makes sense
            let mut path = opts.path.clone();
            path.push(DB_FILENAME);
            if !path.exists() {
                return ax_core::util::formats::ax_err(
                    ActyxOSCode::ERR_PATH_INVALID,
                    format!("path `{}` does not refer to an AX directory", opts.path.display()),
                );
            }

            // lock actyx data directory
            let _lock = lock_working_dir(&opts.path)?;
            println!("locked {:?}", _lock);

            // open settings repo
            let db = Database::new(opts.path).map_err(|e| {
                ActyxOSError::new(ActyxOSCode::ERR_IO, format!("error while opening settings db: {}", e))
            })?;
            let repo = Repository::new(db);

            // make modification
            let scope = Scope::from_str("com.actyx/admin/authorizedUsers").unwrap();
            let mut keys = repo.get_settings(&scope, false)?;
            keys.as_array_mut().unwrap().push(pubkey.to_string().into());
            repo.update_settings(&scope, keys, false)?;

            Ok(())
        });
        Box::new(stream::once(r))
    }
    fn pretty(_result: Self::Output) -> String {
        "OK".to_owned()
    }
}

#[derive(clap::Parser, Clone, Debug)]
/// add own user key to a given AX data directory
pub struct AddKeyOpts {
    /// Path to the `actyx-data` folder you wish to modify
    #[arg(name = "PATH", required = true)]
    path: PathBuf,
    /// Authentication identity (private key).
    /// Can be base64 encoded or a path to a file containing the key,
    /// defaults to `<OS_CONFIG_FOLDER>/keys/id`.
    #[arg(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
    identity: Option<String>,
}
