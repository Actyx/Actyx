#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use async_std::future::timeout;
    use ax_sdk::{
        aql::Query,
        types::{tags, Payload},
    };
    use std::time::Duration;
    use structopt::StructOpt;
    use swarm_cli::{Command, Event};
    use swarm_harness::{HarnessOpts, MachineExt};

    swarm_harness::setup_env()?;
    swarm_harness::run_netsim(HarnessOpts::from_args(), |mut network| async move {
        let (s, r) = network.machines_mut().split_at_mut(1);
        let s = &mut s[0];

        s.send(Command::Append(vec![(
            tags!("a"),
            Payload::from_json_str("\"hello world\"").unwrap(),
        )]));

        for machine in r.iter_mut() {
            loop {
                if let Some(Event::NewListenAddr(_)) = timeout(Duration::from_secs(3), machine.recv()).await? {
                    break;
                }
            }
        }

        for machine in r.iter_mut() {
            s.send(Command::AddAddress(machine.peer_id(), machine.multiaddr()));
        }

        for machine in r.iter_mut() {
            machine.send(Command::SubscribeQuery(Query::parse("FROM 'a'").unwrap()));
        }

        tracing::info!("waiting for events");
        for machine in r.iter_mut() {
            loop {
                if let Some(Event::Result(ev)) = timeout(Duration::from_secs(20), machine.recv()).await? {
                    println!("{} {:?}", machine.id(), ev);
                    break;
                }
            }
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
