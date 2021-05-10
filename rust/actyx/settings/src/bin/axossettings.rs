use anyhow::Context;
use settings::{database::Database, repository::Repository, scope::Scope};
use std::str::FromStr;
use std::{fs::File, io::Read, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "settings",
    about = "Interact with a local ActyxOS settings and schema repository"
)]
struct Opt {
    /// Base path to local settings and schema repository.
    /// Path will be created, if it doesn't exist yet.
    /// Defaults to the current working directory.
    #[structopt(long = "base_dir", parse(from_os_str), default_value = ".")]
    path: PathBuf,
    #[structopt(flatten)]
    cmd: Command,
}
#[derive(StructOpt)]
enum Command {
    #[structopt(name = "setSchema")]
    SetSchema {
        #[structopt(long = "scope", parse(try_from_str = Scope::from_str))]
        scope: Scope,

        #[structopt(long = "schema", help = "Path to schema file")]
        schema: PathBuf,
    },
    #[structopt(name = "deleteSchema")]
    DeleteSchema {
        #[structopt(long = "scope", parse(try_from_str = Scope::from_str))]
        scope: Scope,
    },
    #[structopt(name = "getSettings")]
    GetSettings {
        #[structopt(long = "scope", parse(try_from_str = Scope::from_str), default_value = ".")]
        scope: Scope,
        #[structopt(long = "no_defaults")]
        no_defaults: bool,
    },
    #[structopt(name = "setSettings")]
    SetSettings {
        #[structopt(long = "scope", parse(try_from_str = Scope::from_str))]
        scope: Scope,
        #[structopt(long = "settings", help = "Path to settings file. Defaults to stdin if omitted.")]
        settings: Option<PathBuf>,
        #[structopt(long = "force")]
        force: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let opt: Opt = Opt::from_args();
    let disk_store = Database::new(opt.path).context("Opening settings db")?;
    let mut repo = Repository::new(disk_store).context("Creating settings repo")?;
    match opt.cmd {
        Command::SetSchema { scope, schema } => {
            let schema = serde_json::from_reader(File::open(schema)?)?;
            repo.set_schema(&scope, schema)?;
            println!("Installed schema for scope {}.", scope);
        }
        Command::GetSettings { scope, no_defaults } => {
            let settings = repo.get_settings(&scope, no_defaults)?;
            println!("{}", settings);
        }
        Command::DeleteSchema { scope } => {
            repo.delete_schema(&scope)?;
            println!(
                "Deleted schema at scope {}\n the new settings are:\n{}",
                scope,
                serde_json::to_string_pretty(&repo.get_settings(&scope, true)?)?,
            );
        }
        Command::SetSettings { scope, settings, force } => {
            let settings = match settings {
                Some(settings) => serde_json::from_reader(File::open(settings)?)?,
                None => {
                    let stdin = std::io::stdin();
                    let mut stdin = stdin.lock();
                    let mut line = String::new();

                    while let Ok(n_bytes) = stdin.read_to_string(&mut line) {
                        if n_bytes == 0 {
                            break;
                        }
                    }
                    serde_json::from_str(&line)?
                }
            };
            repo.update_settings(&scope, settings, force)?;
        }
    };
    Ok(())
}
