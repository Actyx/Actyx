use anyhow::{anyhow, Context, Result};
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use futures::StreamExt;
use std::{io::Write, path::PathBuf, time::Duration};
use structopt::StructOpt;
use tokio::sync::mpsc::channel;
use util::version::NodeVersion;

use crate::guided_migration::assess_v2_swarm;
use crate::guided_migration::v1_migrate_sources_and_disseminate;
use crate::guided_migration::v1_overview;
use crate::guided_migration::MigratedSwarm;
use crate::guided_migration::MixedSwarmOverview;

mod guided_migration;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "actyx-v1-migration",
    about = "Actyx v1 to v2 Migration",
    rename_all = "kebab-case"
)]
struct Opts {
    #[structopt(long, env = "ACTYX_PATH")]
    /// Path where to store all the data of the Actyx node.
    /// Defaults to the current working directory
    working_dir: Option<PathBuf>,

    #[structopt(long)]
    version: bool,
}

async fn run(working_dir: PathBuf) -> anyhow::Result<()> {
    let mut out = std::io::stdout();
    let (key_tx, mut key_rx) = channel(128);
    let mut stream = EventStream::new();
    tokio::task::spawn(async move {
        while let Some(k) = stream.next().await {
            if let Ok(Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            })) = &k
            {
                let mut out = std::io::stdout();
                disable_raw_mode().expect("Error while disabling raw mode");
                execute!(out, LeaveAlternateScreen, DisableMouseCapture).expect("Error while resettng screen");
                tracing::error!("Caught ctrl-c. Exiting");
                std::process::exit(1);
            }
            if key_tx.send(k.map_err(Into::into)).await.is_err() {
                break;
            }
        }
    });
    let MixedSwarmOverview {
        to_migrate: sources_to_migrate,
        v1_sources,
    } = v1_overview(&working_dir, &mut out, &mut key_rx).await?;

    tracing::info!(
        "{} sources to migrate ({:?}",
        sources_to_migrate.len(),
        sources_to_migrate
    );
    let MigratedSwarm { store } = v1_migrate_sources_and_disseminate(&working_dir, sources_to_migrate).await?;
    assess_v2_swarm(store, v1_sources, &mut out, &mut key_rx).await?;
    out.flush()?;

    Ok(())
}
fn main() -> Result<()> {
    let Opts { working_dir, version } = Opts::from_args();

    if version {
        println!("Actyx v1 to v2 Migration {}", NodeVersion::get());
    } else {
        if std::env::var("RUST_LOG").is_ok() {
            util::setup_logger();
        }
        // let bind_to: BindTo = Default::default();
        let working_dir = working_dir
            .ok_or_else(|| anyhow!("empty"))
            .or_else(|_| -> Result<_> { Ok(std::env::current_dir()?) })?;

        let _lock = node::lock_working_dir(&working_dir)
            .with_context(|| format!("Exclusive access to {}", working_dir.display()))?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(4)
            .build()?;

        enable_raw_mode()?;
        let mut out = std::io::stdout();
        execute!(out, EnterAlternateScreen, EnableMouseCapture)?;

        let res = rt.block_on(run(working_dir));

        disable_raw_mode()?;
        execute!(out, LeaveAlternateScreen, DisableMouseCapture)?;

        if let Err(err) = res {
            eprintln!("Error during migration: {}", err);
        }

        rt.shutdown_timeout(Duration::from_millis(500));
    }

    Ok(())
}
