use futures::Future;
use libp2p::{
    core::{Endpoint, UpgradeInfo},
    InboundUpgrade, OutboundUpgrade,
};

pub fn from_fn<P, F, C, Fut, Out, Err>(protocol_names: P, fun: F) -> FromFnUpgrade<P, F>
where
    P: IntoIterator + Clone,
    P::Item: AsRef<[u8]>,
    F: FnOnce(C, Endpoint, P::Item) -> Fut,
    Fut: Future<Output = Result<Out, Err>>,
{
    FromFnUpgrade { protocol_names, fun }
}

#[derive(Debug, Clone)]
pub struct FromFnUpgrade<P, F> {
    protocol_names: P,
    fun: F,
}

impl<P, F> UpgradeInfo for FromFnUpgrade<P, F>
where
    P: IntoIterator + Clone,
    P::Item: AsRef<[u8]> + Clone,
{
    type Info = P::Item;
    type InfoIter = P::IntoIter;

    fn protocol_info(&self) -> Self::InfoIter {
        self.protocol_names.clone().into_iter()
    }
}

impl<C, P, F, Fut, Err, Out> InboundUpgrade<C> for FromFnUpgrade<P, F>
where
    P: IntoIterator + Clone,
    P::Item: AsRef<[u8]> + Clone,
    F: FnOnce(C, Endpoint, P::Item) -> Fut,
    Fut: Future<Output = Result<Out, Err>>,
{
    type Output = Out;
    type Error = Err;
    type Future = Fut;

    fn upgrade_inbound(self, sock: C, info: Self::Info) -> Self::Future {
        (self.fun)(sock, Endpoint::Listener, info)
    }
}

impl<C, P, F, Fut, Err, Out> OutboundUpgrade<C> for FromFnUpgrade<P, F>
where
    P: IntoIterator + Clone,
    P::Item: AsRef<[u8]> + Clone,
    F: FnOnce(C, Endpoint, P::Item) -> Fut,
    Fut: Future<Output = Result<Out, Err>>,
{
    type Output = Out;
    type Error = Err;
    type Future = Fut;

    fn upgrade_outbound(self, sock: C, info: Self::Info) -> Self::Future {
        (self.fun)(sock, Endpoint::Dialer, info)
    }
}
