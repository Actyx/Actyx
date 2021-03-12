use super::format::{Message, I, O};
use std::marker::PhantomData;
use tokio::time::Duration;

#[derive(Debug, Default)]
pub struct Counters<D> {
    /// blocks sent or received
    blocks: u64,
    /// block bytes sent or received
    block_bytes: u64,
    /// have requests sent
    have: u64,
    /// want requests sent
    want: u64,
    /// cancel commands sent
    cancel: u64,
    /// block presence responses sent
    block_presences: u64,
    _d: PhantomData<D>,
}

impl<D> Counters<D> {
    fn update(&mut self, message: &Message<D>) {
        self.blocks += message.blocks.len() as u64;
        self.block_bytes += message.blocks.iter().map(|block| block.data().len()).sum::<usize>() as u64;
        self.have += message.have.len() as u64;
        self.want += message.want.len() as u64;
        self.cancel += message.cancel.len() as u64;
        self.block_presences += message.block_presences.len() as u64;
    }
}

/// Contains statistics for the transactions with a peer.
#[derive(Debug, Default)]
pub struct PeerStats {
    /// send stats
    sent: Counters<O>,
    /// recv stats
    recv: Counters<I>,
    /// The number of times this node has promised to have a block and then did not send it
    missed_coming: u64,
}

impl PeerStats {
    /// Creates a new `PeerStats`.
    pub fn new() -> Self {
        Self::default()
    }

    /// update statistics for outgoing blocks
    pub fn update_outgoing_stats(&mut self, message: &Message<O>) {
        self.sent.update(message);
    }

    /// update statistics for incoming blocks
    pub fn update_incoming_stats(&mut self, message: &Message<I>) {
        self.recv.update(message);
    }

    /// log when the other side has missed sending a block in time
    pub fn add_missed_coming(&mut self, _delay: Duration) {
        self.missed_coming += 1;
    }
}
