use futures::{stream::BoxStream, StreamExt};
use std::future::ready;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

pub trait ReceiverExt<T> {
    fn stop_on_error(self) -> BoxStream<'static, T>;
}
impl<T: Send + 'static, E: std::fmt::Debug + Send + 'static> ReceiverExt<T> for Receiver<Result<T, E>> {
    fn stop_on_error(self) -> BoxStream<'static, T> {
        ReceiverStream::new(self)
            .take_while(|x| ready(x.is_ok()))
            .map(|x| x.unwrap())
            .boxed()
    }
}

pub mod future;
pub mod stream;

pub mod prelude {
    pub use crate::stream::AxStreamExt;
}
