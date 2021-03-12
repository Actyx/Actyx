use anyhow::Result;
use futures::{channel::oneshot, StreamExt};
use ipfs_node::IpfsNode;
use std::time::Duration;
use tracing::{debug, span, Level};
use tracing_futures::Instrument;

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn pubsub() -> Result<()> {
    tracing_log::LogTracer::init().unwrap();
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let c1_node = IpfsNode::test().await?;
    let c2_node = IpfsNode::test().await?;
    let c2_address = c2_node.listeners()[0].clone();
    let (c1_snd, c1_rcv) = oneshot::channel::<()>();
    let (c1e_snd, c1e_rcv) = oneshot::channel::<()>();
    let (c2e_snd, c2e_rcv) = oneshot::channel::<()>();

    let c1 = tokio::spawn(
        async move {
            // There seems to be something wrong with subscribing after connection.
            // The nodes does not negotiate the `meshsub` protocol, and even if you
            // do a dummy subscribe to another topic before, they will negotiate the
            // protocol, but there is never any registration of the interest in that
            // topic on the other nodes.
            debug!("c1 sub");
            let mut sub = c1_node.subscribe("/test").unwrap().take(1);
            // Wait for c2 to subscribe as well before we connect
            debug!("c1 wtn");
            c1_rcv.await.unwrap();
            debug!("c1 rdy");
            c1_node.connect(c2_address);
            // Sleep with the signaling so that the connection is done, since I can't figure out
            // how to wait until after the protocol negotiations are all done
            std::thread::sleep(Duration::from_secs(2));
            debug!("c1 pub");
            c1_node.publish("/test", b"c1".to_vec()).unwrap();
            debug!("c1 nxt");
            let mut result: Vec<String> = Vec::new();
            while let Some(value) = sub.next().await {
                let value = String::from_utf8(value).unwrap();
                debug!("c1 <- {:?}", value);
                c1_node.publish("/test", format!("{} c1", value).into()).unwrap();
                std::thread::sleep(Duration::from_millis(500));
                debug!("c1 ->");
                result.push(value);
            }
            assert_eq!(result, vec!["c1 c2"]);
            debug!("c1 end");
            c1e_snd.send(()).unwrap();
            c2e_rcv.await.unwrap();
            debug!("c1 done");
        }
        .instrument(span!(Level::DEBUG, "c1t")),
    );

    let c2 = tokio::spawn(
        async move {
            debug!("c2 sub");
            let mut sub = c2_node.subscribe("/test").unwrap().take(2);
            debug!("c2 rdy");
            c1_snd.send(()).unwrap();
            debug!("c2 nxt");
            let mut result: Vec<String> = Vec::new();
            while let Some(value) = sub.next().await {
                let value = String::from_utf8(value).unwrap();
                debug!("c2 <- {:?}", value);
                c2_node.publish("/test", format!("{} c2", value).into()).unwrap();
                debug!("c2 ->");
                result.push(value);
            }
            assert_eq!(result, vec!["c1", "c1 c2 c1"]);
            debug!("c2 end");
            c2e_snd.send(()).unwrap();
            c1e_rcv.await.unwrap();
            debug!("c2 done");
        }
        .instrument(span!(Level::DEBUG, "c2t")),
    );

    c1.await.unwrap();
    c2.await.unwrap();
    tracing::info!("futures completed");
    Ok(())
}
