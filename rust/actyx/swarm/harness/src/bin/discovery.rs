#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use async_std::future::timeout;
    use std::time::Duration;
    use structopt::StructOpt;
    use swarm_cli::{Command, Event, Multiaddr, PeerId};
    use swarm_harness::{HarnessOpts, MachineExt};

    swarm_harness::setup_env()?;
    swarm_harness::run_netsim(HarnessOpts::from_args(), |mut network| async move {
        let mut peers: Vec<(PeerId, Multiaddr)> = Vec::with_capacity(network.machines().len());
        for machine in network.machines_mut() {
            peers.push((machine.peer_id(), machine.multiaddr()));
        }
        for machine in network.machines_mut() {
            for (peer, addr) in peers.iter() {
                machine.send(Command::AddAddress(*peer, addr.clone()));
            }
        }
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Connected(peer)) = timeout(Duration::from_secs(10), machine.recv()).await? {
                    if peer == peers[0].0 {
                        tracing::info!("connected");
                        break;
                    }
                }
            }
        }
        tracing::info!("fully meshed");
        network.machines_mut()[0].down();
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Disconnected(peer)) = timeout(Duration::from_secs(20), machine.recv()).await? {
                    if peer == peers[0].0 {
                        tracing::info!("disconnected");
                        break;
                    }
                }
            }
        }
        tracing::info!("node gone");
        network.machines_mut()[0].up();
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Connected(peer)) = timeout(Duration::from_secs(20), machine.recv()).await? {
                    if peer == peers[0].0 {
                        tracing::info!("connected");
                        break;
                    }
                }
            }
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
