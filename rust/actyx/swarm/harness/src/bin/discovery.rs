#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use swarm_cli::{Command, Event, Multiaddr, PeerId};
    use swarm_harness::MachineExt;

    swarm_harness::run_netsim(|mut network| async move {
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
                if let Some(Event::Connected(peer)) = machine.recv().await {
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
                if let Some(Event::Disconnected(peer)) = machine.recv().await {
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
                if let Some(Event::Connected(peer)) = machine.recv().await {
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
