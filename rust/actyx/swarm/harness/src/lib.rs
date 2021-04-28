use anyhow::Result;
use futures::prelude::*;
use netsim_embed::{Ipv4Range, Network, NetworkBuilder, Wire};
use std::time::Duration;
use structopt::StructOpt;
use swarm_cli::{Command, Event};
use tempdir::TempDir;

#[derive(StructOpt)]
pub struct HarnessOpts {
    #[structopt(long, default_value = "2")]
    pub n_nodes: usize,

    #[structopt(long, default_value = "0")]
    pub delay_ms: u64,
}

pub fn run_netsim<F, F2>(mut f: F) -> Result<()>
where
    F: FnMut(Network<Command, Event>) -> F2,
    F2: Future<Output = Network<Command, Event>> + Send,
{
    util::setup_logger();
    let opts = HarnessOpts::from_args();
    let swarm_cli = std::env::current_exe()?.parent().unwrap().join("swarm-cli");
    if !swarm_cli.exists() {
        return Err(anyhow::anyhow!(
            "failed to find the swarm-cli binary at {}",
            swarm_cli.display()
        ));
    }
    let temp_dir = TempDir::new("swarm-harness")?;
    netsim_embed::namespace::unshare_user()?;
    async_global_executor::block_on(async move {
        let mut builder = NetworkBuilder::new(Ipv4Range::random_local_subnet());
        for i in 0..opts.n_nodes {
            let swarm_cli = swarm_cli.clone();
            let path = temp_dir.path().join(i.to_string());
            let mut wire = Wire::new();
            wire.set_delay(Duration::from_millis(opts.delay_ms));
            let mut cmd = async_process::Command::new(swarm_cli);
            cmd.arg("--path").arg(path);
            builder.spawn_machine_with_command(wire, cmd);
        }
        let network = builder.spawn();
        let mut network = f(network).await;
        for machine in network.machines_mut() {
            machine.send(Command::Exit).await;
        }
    });
    Ok(())
}
