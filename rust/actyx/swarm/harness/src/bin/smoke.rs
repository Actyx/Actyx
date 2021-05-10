#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyxos_sdk::{tags, Payload};
    use swarm_cli::{Command, Event};

    swarm_harness::run_netsim(|mut network| async move {
        let (s, r) = network.machines_mut().split_at_mut(1);
        for machine in r.iter_mut() {
            loop {
                if let Some(Event::PeerId(peer)) = machine.recv().await {
                    let addr = format!("/ip4/{}/tcp/30000", machine.addr()).parse().unwrap();
                    s[0].send(Command::AddAddress(peer, addr)).await;
                    break;
                }
            }
        }
        for _ in r {
            loop {
                if let Some(Event::Subscribed(_, topic)) = s[0].recv().await {
                    tracing::info!("subscribed {}", topic);
                    break;
                }
            }
        }
        for machine in &mut network.machines_mut()[1..] {
            machine.send(Command::Query("FROM 'a'".parse().unwrap())).await;
        }
        network
            .machine(0)
            .send(Command::Append(
                0.into(),
                vec![(tags!("a"), Payload::from_json_str("\"hello world\"").unwrap())],
            ))
            .await;
        tracing::info!("waiting for events");
        for machine in &mut network.machines_mut()[1..] {
            loop {
                if let Some(Event::Result(ev)) = machine.recv().await {
                    println!("{:?}", ev);
                    break;
                }
            }
        }
        network
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
