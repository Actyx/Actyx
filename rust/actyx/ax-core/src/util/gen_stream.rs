use futures::{pin_mut, stream::FusedStream, Future, Stream};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct GenStream<Item, F: Future<Output = Item>> {
    gen: Gen<Item, (), F>,
    completed: bool,
}

impl<Item, F: Future<Output = Item>> GenStream<Item, F> {
    pub fn new(f: impl FnOnce(Co<Item>) -> F) -> Self {
        Self {
            gen: Gen::new(f),
            completed: false,
        }
    }
}

impl<Item, F: Future<Output = Item>> Stream for GenStream<Item, F> {
    type Item = Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.completed {
            return Poll::Ready(None);
        }
        let next = {
            let fut = self.gen.async_resume();
            pin_mut!(fut);
            fut.poll(cx)
        };
        match next {
            Poll::Ready(GeneratorState::Yielded(v)) => Poll::Ready(Some(v)),
            Poll::Ready(GeneratorState::Complete(v)) => {
                self.completed = true;
                Poll::Ready(Some(v))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<Item, F: Future<Output = Item>> FusedStream for GenStream<Item, F> {
    fn is_terminated(&self) -> bool {
        self.completed
    }
}
