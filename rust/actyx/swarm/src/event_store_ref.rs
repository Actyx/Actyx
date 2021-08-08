use crate::{
    event_store::{EventStore, PersistenceMeta},
    BanyanStore, SwarmOffsets,
};
use actyx_sdk::{language::TagExpr, AppId, Event, OffsetMap, Payload, TagSet};
use futures::{Future, Stream, StreamExt};
use parking_lot::Mutex;
use std::{
    collections::BTreeMap,
    future::ready,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display(fmt = "Event store was stopped while request was queued or running.")]
    Aborted,
    #[display(fmt = "Channel towards event store is overloaded.")]
    Overload,
    #[display(fmt = "Query bounds out of range: upper bound must be within the known present.")]
    InvalidUpperBounds,
    #[display(fmt = "AQL Error: {}", _0)]
    TagExprError(TagExprError),
}

impl From<super::event_store::Error> for Error {
    fn from(x: super::event_store::Error) -> Self {
        match x {
            crate::event_store::Error::InvalidUpperBounds => Error::InvalidUpperBounds,
            crate::event_store::Error::TagExprError(e) => Error::TagExprError(e),
        }
    }
}

impl<T> From<crossbeam::channel::TrySendError<T>> for Error {
    fn from(e: crossbeam::channel::TrySendError<T>) -> Self {
        match e {
            crossbeam::channel::TrySendError::Full(_) => Error::Overload,
            crossbeam::channel::TrySendError::Disconnected(_) => Error::Aborted,
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::TrySendError<T>> for Error {
    fn from(e: tokio::sync::mpsc::error::TrySendError<T>) -> Self {
        match e {
            tokio::sync::mpsc::error::TrySendError::Full(_) => Error::Overload,
            tokio::sync::mpsc::error::TrySendError::Closed(_) => Error::Aborted,
        }
    }
}

#[derive(Clone)]
pub struct EventStoreRef {
    tx: Arc<dyn Fn(EventStoreRequest) -> Result<(), Error> + Send + Sync + 'static>,
}

type OneShot<T> = oneshot::Sender<Result<T, Error>>;
type StreamOf<T> = mpsc::Receiver<Result<T, Error>>;
type StreamTo<T> = mpsc::Sender<Result<T, Error>>;

#[derive(derive_more::Display)]
pub enum EventStoreRequest {
    #[display(fmt = "Offsets")]
    Offsets { reply: OneShot<SwarmOffsets> },
    #[display(fmt = "Persist({}, {})", app_id, "events.len()")]
    Persist {
        app_id: AppId,
        events: Vec<(TagSet, Payload)>,
        reply: OneShot<Vec<PersistenceMeta>>,
    },
    #[display(fmt = "Bounded({}, per_stream={})", tag_expr, per_stream)]
    BoundedForward {
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
        per_stream: bool,
        reply: OneShot<StreamOf<Event<Payload>>>,
    },
    #[display(fmt = "Backward({})", tag_expr)]
    BoundedBackward {
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
        reply: OneShot<StreamOf<Event<Payload>>>,
    },
    #[display(fmt = "Unbounded({})", tag_expr)]
    UnboundedForward {
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
        reply: OneShot<StreamOf<Event<Payload>>>,
    },
}

use trees::query::TagExprError;
use EventStoreRequest::*;
impl EventStoreRef {
    pub fn new(f: impl Fn(EventStoreRequest) -> Result<(), Error> + Send + Sync + 'static) -> Self {
        Self { tx: Arc::new(f) }
    }

    pub async fn offsets(&self) -> Result<SwarmOffsets, Error> {
        let (reply, rx) = oneshot::channel();
        (self.tx)(Offsets { reply })?;
        rx.await.my_err()?
    }

    pub async fn persist(&self, app_id: AppId, events: Vec<(TagSet, Payload)>) -> Result<Vec<PersistenceMeta>, Error> {
        let (reply, rx) = oneshot::channel();
        (self.tx)(Persist { app_id, events, reply })?;
        rx.await.my_err()?
    }

    pub async fn bounded_forward(
        &self,
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
        per_stream: bool,
    ) -> Result<mpsc::Receiver<Result<Event<Payload>, Error>>, Error> {
        let (reply, rx) = oneshot::channel();
        (self.tx)(BoundedForward {
            tag_expr,
            from_offsets_excluding,
            to_offsets_including,
            per_stream,
            reply,
        })?;
        rx.await.my_err()?
    }

    pub async fn bounded_backward(
        &self,
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
    ) -> Result<mpsc::Receiver<Result<Event<Payload>, Error>>, Error> {
        let (reply, rx) = oneshot::channel();
        (self.tx)(BoundedBackward {
            tag_expr,
            from_offsets_excluding,
            to_offsets_including,
            reply,
        })?;
        rx.await.my_err()?
    }

    pub async fn unbounded_forward(
        &self,
        tag_expr: TagExpr,
        from_offsets_excluding: OffsetMap,
    ) -> Result<mpsc::Receiver<Result<Event<Payload>, Error>>, Error> {
        let (reply, rx) = oneshot::channel();
        (self.tx)(UnboundedForward {
            tag_expr,
            from_offsets_excluding,
            reply,
        })?;
        rx.await.my_err()?
    }
}

trait MyErr<T> {
    fn my_err(self) -> Result<T, Error>;
}
impl<T, U> MyErr<T> for Result<T, mpsc::error::SendError<U>> {
    fn my_err(self) -> Result<T, Error> {
        self.map_err(|_| Error::Aborted)
    }
}
impl<T, U> MyErr<T> for Result<T, mpsc::error::TrySendError<U>> {
    fn my_err(self) -> Result<T, Error> {
        self.map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => Error::Overload,
            mpsc::error::TrySendError::Closed(_) => Error::Aborted,
        })
    }
}
impl<T> MyErr<T> for Result<T, oneshot::error::RecvError> {
    fn my_err(self) -> Result<T, Error> {
        self.map_err(|_| Error::Aborted)
    }
}

pub struct EventStoreHandler {
    store: EventStore,
    state: Arc<State>,
}

type StreamInfo = (JoinHandle<()>, Option<StreamTo<Event<Payload>>>);

#[derive(Default)]
struct State {
    persist: AtomicUsize,
    stream_id: AtomicUsize,
    stream: Mutex<BTreeMap<usize, StreamInfo>>,
}

impl EventStoreHandler {
    pub fn new(store: BanyanStore) -> Self {
        Self {
            store: EventStore::new(store),
            state: Arc::new(State::default()),
        }
    }

    /// Handle the given request, spawning tasks on the given Runtime as needed.
    pub fn handle(&mut self, request: EventStoreRequest, runtime: &Handle) {
        match request {
            Offsets { reply } => {
                let _ = reply.send(Ok(self.store.current_offsets()));
            }
            Persist { app_id, events, reply } => {
                let store = self.store.clone();
                self.state.persist.fetch_add(1, Ordering::Relaxed);
                let state = self.state.clone();
                runtime.spawn(async move {
                    let n = events.len();
                    let _ = reply.send(store.persist(app_id, events).await.map_err(move |e| {
                        tracing::error!("failed to persist {} events: {:#}", n, e);
                        Error::Aborted
                    }));
                    state.persist.fetch_sub(1, Ordering::Relaxed);
                });
            }
            BoundedForward {
                tag_expr,
                from_offsets_excluding,
                to_offsets_including,
                per_stream,
                reply,
            } => {
                let store = self.store.clone();
                self.stream(reply, runtime, move || async move {
                    if per_stream {
                        store
                            .bounded_forward_per_stream(&tag_expr, from_offsets_excluding, to_offsets_including)
                            .await
                            .map(|s| s.boxed())
                    } else {
                        store
                            .bounded_forward(&tag_expr, from_offsets_excluding, to_offsets_including)
                            .await
                            .map(|s| s.boxed())
                    }
                });
            }
            BoundedBackward {
                tag_expr,
                from_offsets_excluding,
                to_offsets_including,
                reply,
            } => {
                let store = self.store.clone();
                self.stream(reply, runtime, move || async move {
                    store
                        .bounded_backward(&tag_expr, from_offsets_excluding, to_offsets_including)
                        .await
                });
            }
            UnboundedForward {
                tag_expr,
                from_offsets_excluding,
                reply,
            } => {
                let store = self.store.clone();
                self.stream(reply, runtime, move || {
                    ready(store.unbounded_forward_per_stream(&tag_expr, from_offsets_excluding))
                });
            }
        }
    }

    fn stream<F, Fut, S>(&mut self, reply: OneShot<StreamOf<Event<Payload>>>, runtime: &Handle, f: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<S, super::event_store::Error>> + Send + 'static,
        S: Stream<Item = Event<Payload>> + Unpin + Send + 'static,
    {
        let state = self.state.clone();
        let id = state.stream_id.fetch_add(1, Ordering::Relaxed);
        let (start, started) = oneshot::channel();
        let handle = runtime.spawn(async move {
            tracing::debug!("stream {} initiated", id);
            match f().await {
                Ok(mut s) => {
                    tracing::debug!("stream {} starting", id);
                    let (tx, rx) = mpsc::channel(100);
                    let _ = started.await;
                    let doit = if let Some(x) = state.stream.lock().get_mut(&id) {
                        x.1 = Some(tx.clone());
                        true
                    } else {
                        false
                    }; // lock is dropped here
                    tracing::debug!("stream {} started {}", id, doit);
                    if doit && reply.send(Ok(rx)).is_ok() {
                        while let Some(event) = s.next().await {
                            tracing::trace!("stream {} got {}/{}", id, event.key.lamport, event.key.stream);
                            if tx.send(Ok(event)).await.is_err() {
                                // stream recipient has lost interest
                                tracing::debug!("stream {} aborted", id);
                                break;
                            }
                            tracing::trace!("stream {} sent", id);
                        }
                        tracing::debug!("stream {} ended", id);
                    }
                }
                Err(e) => {
                    let _ = reply.send(Err(e.into()));
                }
            }
            // need to drop the other stream sender to end the stream
            state.stream.lock().remove(&id);
        });
        self.state.stream.lock().insert(id, (handle, None));
        let _ = start.send(());
    }
}

impl Drop for EventStoreHandler {
    fn drop(&mut self) {
        let mut streams = self.state.stream.lock();
        tracing::info!(
            "stopping store with {} ongoing persist calls and {} ongoing queries",
            self.state.persist.load(Ordering::Relaxed),
            streams.len()
        );
        for (_id, (handle, stream)) in streams.iter() {
            handle.abort();
            if let Some(stream) = stream {
                let _ = stream.try_send(Err(Error::Aborted));
                // the stream receiver will soon be dropped, leading to the task to end as well
            }
        }
        streams.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_string() {
        assert_eq!(
            Error::Aborted.to_string(),
            "Event store was stopped while request was queued or running."
        );
        assert_eq!(
            Error::Overload.to_string(),
            "Channel towards event store is overloaded."
        );
        assert_eq!(
            Error::InvalidUpperBounds.to_string(),
            "Query bounds out of range: upper bound must be within the known present."
        );
    }
}
