use crossbeam::{channel::TrySendError, queue::ArrayQueue};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::{span, Event, Metadata, Subscriber};

pub struct WrappingSubscriber<S: Subscriber, T> {
    subscriber: S,
    queue: Arc<ArrayQueue<T>>,
}
pub trait ConvertEvent {
    fn convert(ev: &Event<'_>) -> Self;
}

impl<S: Subscriber, T: 'static + ConvertEvent> WrappingSubscriber<S, T> {
    pub fn new(subscriber: S, queue: Arc<ArrayQueue<T>>) -> Self {
        Self { subscriber, queue }
    }
}

impl<'a, S: Subscriber, T> Subscriber for WrappingSubscriber<S, T>
where
    T: ConvertEvent + 'static,
{
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.subscriber.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.subscriber.new_span(span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.subscriber.record(span, values)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.subscriber.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        if self.queue.push(T::convert(event)).is_err() {
            eprintln!("Queue full, discarding log event ..");
        }
        self.subscriber.event(event)
    }

    fn enter(&self, span: &span::Id) {
        self.subscriber.enter(span)
    }

    fn exit(&self, span: &span::Id) {
        self.subscriber.exit(span)
    }
}

use crossbeam::channel::Sender;
pub struct WrappingSubscriber2<S: Subscriber, T> {
    subscriber: S,
    tx: Sender<T>,
    ignore_tag_prefix: String,
    throttle: Arc<Mutex<Option<(Instant, usize)>>>,
}

impl<S: Subscriber, T: 'static + ConvertEvent> WrappingSubscriber2<S, T> {
    pub fn new(subscriber: S, tx: Sender<T>, ignore_tag_prefix: String) -> Self {
        Self {
            subscriber,
            tx,
            ignore_tag_prefix,
            throttle: Arc::new(Mutex::new(None)),
        }
    }
}

impl<'a, S: Subscriber, T> Subscriber for WrappingSubscriber2<S, T>
where
    T: ConvertEvent + 'static,
{
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.subscriber.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.subscriber.new_span(span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.subscriber.record(span, values)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.subscriber.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        if !event.metadata().target().starts_with(&self.ignore_tag_prefix) {
            let mut guard = self.throttle.lock().unwrap();
            match self.tx.try_send(T::convert(event)) {
                Ok(_) => {
                    if let Some((time, dropped)) = guard.take() {
                        eprintln!(
                            "Channel is available again. Dropped {} log events in the last {} ms.",
                            dropped,
                            time.elapsed().as_micros()
                        );
                    }
                }
                Err(e) => {
                    let dropped_cnt = guard.map(|x| x.1).unwrap_or(0);
                    *guard = Some((Instant::now(), dropped_cnt + 1));
                    if dropped_cnt == 0 {
                        match e {
                            TrySendError::Full(_) => eprintln!(
                                "Channel is full ({}) -- logsvcd can't catch up, dropping log events.",
                                self.tx.len()
                            ),
                            TrySendError::Disconnected(_) => eprintln!(
                                "Error sending logs via attached channel. Channel is disconnected -- Logging dead."
                            ),
                        }
                    }
                }
            };
        }
        self.subscriber.event(event)
    }

    fn enter(&self, span: &span::Id) {
        self.subscriber.enter(span)
    }

    fn exit(&self, span: &span::Id) {
        self.subscriber.exit(span)
    }
}
