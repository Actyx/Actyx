use std::time::Duration;

#[cfg(test)]
pub fn wait_for<T: Send + 'static>(fut: impl futures::future::Future<Output = T> + Unpin + Send + 'static) -> T {
    use futures::FutureExt;
    let rt = tokio::runtime::Runtime::new().expect("Could not start tokio runtime");
    rt.block_on(fut.map(Result::<T, ()>::Ok)).expect("boo")
}

pub async fn delay_ms<T: Send>(millis: u64, value: T) -> T {
    tokio::time::sleep(Duration::from_millis(millis)).await;
    value
}
