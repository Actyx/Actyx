#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::{language::Query, tags, Payload};
    use async_std::future::timeout;
    use std::time::Duration;
    use structopt::StructOpt;
    use swarm_cli::{Command, Event};
    use swarm_harness::{HarnessOpts, MachineExt, MultiaddrExt};

    swarm_harness::setup_env()?;
    swarm_harness::run_netsim(HarnessOpts::from_args(), |mut network| async move {
        tracing::info!("waiting for new listen addresses");
        for machine in network.machines_mut() {
            loop {
                match timeout(Duration::from_secs(3), machine.recv()).await? {
                    Some(Event::NewListenAddr(addr)) => {
                        if !addr.is_loopback() {
                            break;
                        }
                    }
                    ev => tracing::info!("{:?}", ev),
                }
            }
        }
        let (s, r) = network.machines_mut().split_at_mut(1);
        let s = &mut s[0];
        for machine in r.iter_mut() {
            s.send(Command::AddAddress(machine.peer_id(), machine.multiaddr()));
        }
        tracing::info!("waiting for subscriptions");
        for _ in r.iter_mut() {
            loop {
                if let Some(Event::Subscribed(peer_id, topic)) = timeout(Duration::from_secs(3), s.recv()).await? {
                    tracing::info!("subscribed {} {}", topic, peer_id);
                    break;
                }
            }
        }
        for _ in r.iter_mut() {
            loop {
                if let Some(Event::Subscribed(peer_id, topic)) = timeout(Duration::from_secs(3), s.recv()).await? {
                    tracing::info!("subscribed {} {}", topic, peer_id);
                    break;
                }
            }
        }
        for machine in r.iter_mut() {
            machine.send(Command::SubscribeQuery(Query::parse("FROM 'a'").unwrap()));
        }
        s.send(Command::Append(vec![(
            tags!("a"),
            Payload::from_json_str("\"hello world\"").unwrap(),
        )]));
        tracing::info!("waiting for events");
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Result(ev)) = timeout(Duration::from_secs(20), machine.recv()).await? {
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
