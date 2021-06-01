#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyxos_sdk::{tags, Payload};
    use structopt::StructOpt;
    use swarm_cli::{Command, Event};
    use swarm_harness::{HarnessOpts, MachineExt, MultiaddrExt};

    swarm_harness::run_netsim(HarnessOpts::from_args(), |mut network| async move {
        for machine in network.machines_mut() {
            loop {
                if let Some(Event::NewListenAddr(addr)) = machine.recv().await {
                    if !addr.is_loopback() {
                        break;
                    }
                }
            }
        }
        let (s, r) = network.machines_mut().split_at_mut(1);
        let s = &mut s[0];
        for machine in r.iter_mut() {
            s.send(Command::AddAddress(machine.peer_id(), machine.multiaddr()));
        }
        for _ in r.iter_mut() {
            loop {
                if let Some(Event::Subscribed(_, topic)) = s.recv().await {
                    tracing::info!("subscribed {}", topic);
                    break;
                }
            }
        }
        for machine in r.iter_mut() {
            machine.send(Command::SubscribeQuery("FROM 'a'".parse().unwrap()));
        }
        s.send(Command::Append(
            0.into(),
            vec![(tags!("a"), Payload::from_json_str("\"hello world\"").unwrap())],
        ));
        tracing::info!("waiting for events");
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Result(ev)) = machine.recv().await {
                    println!("{:?}", ev);
                    break;
                }
            }
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
