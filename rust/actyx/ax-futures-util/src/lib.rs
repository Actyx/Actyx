use futures::{stream::BoxStream, StreamExt};
use std::future::ready;
use stream::AxStreamExt;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

pub trait ReceiverExt<T> {
    fn stop_on_error(self) -> BoxStream<'static, T>;
}
impl<T: Send + 'static, E: Send + 'static> ReceiverExt<Result<T, E>> for Receiver<Result<T, E>> {
    fn stop_on_error(self) -> BoxStream<'static, Result<T, E>> {
        ReceiverStream::new(self)
            .take_until_condition(|x| ready(x.is_err()))
            .boxed()
    }
}

pub mod future;
pub mod stream;

pub mod prelude {
    pub use crate::stream::AxStreamExt;
}
