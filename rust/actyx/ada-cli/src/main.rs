use anyhow::Result;
use clap::{App, Arg, ArgGroup, ArgMatches};
use swarm::BanyanStore;
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

    let verbosity = cmd_args::get_verbosity(&matches);
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new(verbosity))
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let store = BanyanStore::test("ada-cli").await?;
    run_app(app, matches, store).await?;

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
        .subcommand(cmd::pubsub_connect::args())
        .subcommand(cmd::monitor_pubsub::args())
        .subcommand(cmd::copy_pubsub::args())
        .subcommand(cmd::pubsub_to_pg::args())
}

async fn run_app(mut app: App<'_, '_>, matches: ArgMatches<'_>, store: BanyanStore) -> Result<()> {
    let subcommands: Vec<Box<dyn cmd::Command>> = vec![
        Box::new(cmd::pubsub_connect::Cmd),
        Box::new(cmd::monitor_pubsub::Cmd),
        Box::new(cmd::copy_pubsub::Cmd),
        Box::new(cmd::pubsub_to_pg::Cmd),
    ];

    for subcommand in subcommands {
        if let Some(matches) = matches.subcommand_matches(subcommand.name()) {
            subcommand.run(&matches, store).await?;
            return Ok(());
        }
    }

    app.print_help().unwrap();
    println!();
    Ok(())
}
