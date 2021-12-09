use crate::cmd::KeyPathWrapper;
use crate::{cmd::AxCliCommand, private_key::AxPrivateKey};
use futures::{stream, Stream};
use settings::{Database, Repository, Scope, DB_FILENAME};
use std::{convert::TryFrom, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use util::{
    ax_bail,
    formats::{ax_err, ActyxOSCode, ActyxOSError, ActyxOSResult},
};

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
            "Actyx directory is in use, please stop Actyx first!".to_owned(),
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
            let privkey = AxPrivateKey::try_from(&opts.identity)?;
            let pubkey = privkey.to_public();

            // check that the path makes sense
            let mut path = opts.path.clone();
            path.push(DB_FILENAME);
            if !path.exists() {
                ax_bail!(
                    ActyxOSCode::ERR_PATH_INVALID,
                    "path `{}` does not refer to an Actyx directory",
                    opts.path.display()
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

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// add own user key to a given Actyx data directory
pub struct AddKeyOpts {
    #[structopt(name = "PATH", required = true)]
    /// Path to the `actyx-data` folder you wish to modify
    path: PathBuf,
    #[structopt(short, long, value_name = "FILE")]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
}
