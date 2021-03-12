use anyhow::Result;
use futures::channel::oneshot;
use ipfs_node::{Block, Cid, IpfsNode};
use libipld::multihash::{Code, MultihashDigest};
use std::time::Duration;
use tracing::*;
use tracing_futures::Instrument;

fn create_block<T: AsRef<[u8]>>(data: T) -> Block {
    let hash = Code::Sha2_256.digest(data.as_ref());
    let cid = Cid::new_v1(libipld::raw::RawCodec.into(), hash);
    Block::new_unchecked(cid, data.as_ref().to_vec())
}

#[tokio::test(flavor = "multi_thread")]
async fn bitswap_two_peers() -> Result<()> {
    tracing_log::LogTracer::init().unwrap();
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    debug!("Starting bitswap test");
    let c1_node = IpfsNode::test().await?;
    let c2_node = IpfsNode::test().await?;
    tokio::time::sleep(Duration::from_millis(10)).await;
    let c2_address = c2_node.listeners()[0].clone();
    let (c1_snd, c1_rcv) = oneshot::channel();
    let (c2_snd, c2_rcv) = oneshot::channel();
    let (c1e_snd, c1e_rcv) = oneshot::channel();
    let (c2e_snd, c2e_rcv) = oneshot::channel();

    let block1 = create_block(b"data_c1_c2_1");
    let block2 = create_block(b"data_c2_c1_1");

    let test1 = async {
        debug!("c1 put 1");
        c1_node.insert(block1.clone()).await.unwrap();
        // Wait for c2 to put a block into the store as well
        debug!("c1 wtn");
        c1_rcv.await.unwrap();
        debug!("c1 cnx");
        c1_node.connect(c2_address);
        // Sleep with the signaling so that the connection is done, since I can't figure out
        // how to wait until after the protocol negotiations are all done
        std::thread::sleep(Duration::from_secs(2));

        debug!("c1 snd");
        c2_snd.send(()).unwrap();
        debug!("c1 rdy");
        let block = c1_node.fetch(block2.cid()).await.unwrap();
        assert_eq!(block, block2);
        debug!("c1 end");
        c1e_snd.send(()).unwrap();
        c2e_rcv.await.unwrap();
        debug!("c1 done");
    }
    .instrument(span!(Level::DEBUG, "c1t"));

    let test2 = async {
        debug!("c2 put 1");
        c2_node.insert(block2.clone()).await.unwrap();
        debug!("c2 snd");
        c1_snd.send(()).unwrap();
        debug!("c2 wtn");
        c2_rcv.await.unwrap();
        debug!("c2 rdy");
        let block = c2_node.fetch(block1.cid()).await.unwrap();
        assert_eq!(block, block1);
        debug!("c2 end");
        c2e_snd.send(()).unwrap();
        c1e_rcv.await.unwrap();
        debug!("c2 done");
    }
    .instrument(span!(Level::DEBUG, "c2t"));

    let (_, _) = futures::future::join(test1, test2).await;
    debug!("futures completed");
    Ok(())
}
