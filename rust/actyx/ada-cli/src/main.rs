use anyhow::Result;
use ax_config::StoreConfig;
use clap::{App, Arg, ArgGroup, ArgMatches};
use std::convert::TryFrom;
use store_core::BanyanStore;
use tracing_subscriber::EnvFilter;

mod cmd;
mod cmd_args;

#[tokio::main]
async fn main() -> Result<()> {
    let app = build_cli();
    let matches = app.clone().get_matches();

    let should_exit = cmd_args::handle_completion("ada-cli", &matches, &build_cli);
    if should_exit {
        return Ok(());
    }

    let mut config = StoreConfig::new("ada-cli".to_string());
    config.log_verbosity = TryFrom::try_from(cmd_args::get_verbosity(&matches).as_str())?;

    let filter = EnvFilter::new(config.log_verbosity.to_string());
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let store = BanyanStore::from_axconfig(config.clone()).await?;
    run_app(app, matches, config, store).await?;

    Ok(())
}

fn build_cli() -> App<'static, 'static> {
    cmd_args::add_common_options(App::new("ada-cli").about("Command line client for Actyx IPFS swarms"))
        .arg(
            Arg::with_name("api")
                .help("Multiaddr or port to connect to the IPFS API (default /ip4/127.0.0.1/tcp/5001)")
                .long("api")
                .short("a")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bootstrap")
                .help("List of bootstrap multiaddrs when running in full_node mode")
                .long("bootstrap")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("listen")
                .help("List of multiaddrs to listen on when running in full_node mode")
                .long("listen")
                .takes_value(true),
        )
        .group(ArgGroup::with_name("clients").args(&["api", "use_dump"]))
        .subcommand(cmd::snapshot_listen::args())
        .subcommand(cmd::logs::args())
        .subcommand(cmd::logs_to_loki::args())
        .subcommand(cmd::balena_logs_to_pubsub::args())
        .subcommand(cmd::pubsub_connect::args())
        .subcommand(cmd::monitor_pubsub::args())
        .subcommand(cmd::copy_pubsub::args())
        .subcommand(cmd::pubsub_to_pg::args())
}

async fn run_app(mut app: App<'_, '_>, matches: ArgMatches<'_>, config: StoreConfig, store: BanyanStore) -> Result<()> {
    let subcommands: Vec<Box<dyn cmd::Command>> = vec![
        Box::new(cmd::snapshot_listen::Cmd),
        Box::new(cmd::logs::Cmd),
        Box::new(cmd::logs_to_loki::Cmd),
        Box::new(cmd::balena_logs_to_pubsub::Cmd),
        Box::new(cmd::pubsub_connect::Cmd),
        Box::new(cmd::monitor_pubsub::Cmd),
        Box::new(cmd::copy_pubsub::Cmd),
        Box::new(cmd::pubsub_to_pg::Cmd),
    ];

    for subcommand in subcommands {
        if let Some(matches) = matches.subcommand_matches(subcommand.name()) {
            subcommand.run(&matches, config, store).await?;
            return Ok(());
        }
    }

    app.print_help().unwrap();
    println!();
    Ok(())
}
