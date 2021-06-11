use futures::Stream;
use pin_project_lite::pin_project;

pin_project! {
    pub struct InspectPoll<S, F> {
        #[pin]
        stream: S,
        func: F,
    }
}

impl<S, F> InspectPoll<S, F> {
    pub fn new(stream: S, func: F) -> Self {
        Self { stream, func }
    }
}

impl<S: Stream + Unpin, F: FnMut(&futures::task::Poll<Option<S::Item>>)> Stream for InspectPoll<S, F> {
    type Item = S::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let res = this.stream.poll_next(cx);
        (this.func)(&res);
        res
    }
}
