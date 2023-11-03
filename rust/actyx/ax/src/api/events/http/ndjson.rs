use std::error::Error;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use self::sealed::NdjsonError;
use futures::{
    future,
    stream::{Stream, TryStream, TryStreamExt},
    StreamExt,
};
use pin_project::pin_project;
use tokio::time::{self, Sleep};

type Bytes = Vec<u8>;
static DELIM: &[u8] = b"\n";

pub fn reply<S>(event_stream: S) -> impl warp::Reply
where
    S: TryStream<Ok = Bytes> + Send + 'static,
    S::Error: Error + Send + Sync + 'static,
{
    NdjsonReply { event_stream }
}

struct NdjsonReply<S> {
    event_stream: S,
}

impl<S> warp::Reply for NdjsonReply<S>
where
    S: TryStream<Ok = Bytes> + Send + 'static,
    S::Error: Error + Send + Sync + 'static,
{
    #[inline]
    fn into_response(self) -> warp::reply::Response {
        let body_stream = self
            .event_stream
            .map_err(|error| {
                tracing::error!(?error, "Error converting to Ndjson");
                NdjsonError
            })
            .into_stream()
            .and_then(|event| future::ready(Ok(event)));

        let mut res = warp::reply::Response::new(hyper::Body::wrap_stream(body_stream));
        res.headers_mut().insert(
            warp::http::header::CONTENT_TYPE,
            warp::http::header::HeaderValue::from_static("application/x-ndjson"),
        );
        res
    }
}

/// Configure the interval between keep-alive messages, the content
/// of each message, and the associated stream.
#[derive(Debug)]
pub struct KeepAlive {
    max_interval: Duration,
    delimiter: Bytes,
    writer_capacity: usize,
}

impl KeepAlive {
    /// Customize the interval between keep-alive messages.
    ///
    /// Default is 15 seconds.
    #[allow(dead_code)]
    pub fn interval(mut self, time: Duration) -> Self {
        self.max_interval = time;
        self
    }

    /// Customize the delimiter and keep-alive value.
    ///
    /// Default is `\n`.
    #[allow(dead_code)]
    pub fn delimiter(mut self, delim: Bytes) -> Self {
        self.delimiter = delim;
        self
    }

    /// Customize the capacity of the serialization buffer.
    ///
    /// Default is 128.
    #[allow(dead_code)]
    pub fn writer_capacity(mut self, capacity: usize) -> Self {
        self.writer_capacity = capacity;
        self
    }

    /// Wrap a response stream with keep-alive functionality.
    ///
    /// See [`keep_alive`](keep_alive) for more.
    pub fn stream<S>(
        self,
        event_stream: impl Stream<Item = S> + Send + 'static,
    ) -> impl TryStream<Ok = Bytes, Error = impl Error + Send + Sync + 'static> + Send + 'static
    where
        S: serde::Serialize + Send + 'static,
    {
        let alive_timer = time::sleep(self.max_interval);

        let delimiter = self.delimiter.clone();
        let capacity = self.writer_capacity;
        let event_stream = event_stream.map(move |e| {
            let mut writer = Vec::with_capacity(capacity);
            serde_json::to_writer(&mut writer, &e)?;
            writer.extend(&delimiter);
            Ok::<Bytes, serde_json::error::Error>(writer)
        });

        NdjsonKeepAlive {
            event_stream,
            max_interval: self.max_interval,
            delimiter: self.delimiter,
            alive_timer,
        }
    }
}

#[pin_project]
struct NdjsonKeepAlive<S> {
    #[pin]
    event_stream: S,
    max_interval: Duration,
    delimiter: Bytes,
    #[pin]
    alive_timer: Sleep,
}

pub fn keep_alive() -> KeepAlive {
    KeepAlive {
        max_interval: Duration::from_secs(15),
        delimiter: DELIM.to_vec(),
        writer_capacity: 128,
    }
}

impl<S> Stream for NdjsonKeepAlive<S>
where
    S: TryStream<Ok = Bytes> + Send + 'static,
    S::Error: Error + Send + Sync + 'static,
{
    type Item = Result<Bytes, NdjsonError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut pin = self.project();
        match pin.event_stream.try_poll_next(cx) {
            Poll::Pending => {
                match pin.alive_timer.as_mut().poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(_) => {
                        // restart timer
                        pin.alive_timer.reset(tokio::time::Instant::now() + *pin.max_interval);
                        Poll::Ready(Some(Ok(pin.delimiter.clone())))
                    }
                }
            }
            Poll::Ready(Some(Ok(event))) => {
                // restart timer
                pin.alive_timer.reset(tokio::time::Instant::now() + *pin.max_interval);
                Poll::Ready(Some(Ok(event)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(error))) => {
                tracing::error!("ndjson error: {}", error);
                Poll::Ready(Some(Err(NdjsonError)))
            }
        }
    }
}

mod sealed {
    use super::*;

    #[derive(Debug)]
    pub struct NdjsonError;

    impl Display for NdjsonError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            write!(f, "ndjson error")
        }
    }

    impl Error for NdjsonError {}
}
