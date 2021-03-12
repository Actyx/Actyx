use futures::{Future, TryFutureExt};
use hyper::{server::Server, service::make_service_fn};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::convert::Infallible;
use std::net::SocketAddr;
use warp::filters::BoxedFilter;
use warp::Reply;

/// Creates a `std::net::TcpListener` for the given `addr`. This also sets the `SO_REUSEADDR` flag
/// on the socket.
fn make_listener<T: Into<SocketAddr>>(addr: T) -> Result<std::net::TcpListener, anyhow::Error> {
    let addr = addr.into();
    let is_ipv4 = addr.is_ipv4();
    let domain = if is_ipv4 { Domain::ipv4() } else { Domain::ipv6() };
    let addr: SockAddr = addr.into();
    let socket = Socket::new(domain, Type::stream(), Some(Protocol::tcp()))?;
    socket.set_reuse_address(true)?;
    // This effectively disables dual-stack usage. The standard behaviour
    // without enabling this flag varies depending on the operating system's IP
    // address stack implementation. Some support IPv4-mapped IPv6 addresses
    // (e.g. Linux and newer versions of Windows) so a single IPv6 address would
    // support IPv4-mapped addresses too. Others do not (e.g. OpenBSD). If they
    // do, then some support them by default (e.g. Linux) and some do not (e.g.
    // Windows). Meaning, that this disables IPv4-mapped IPv6 addresses, hence
    // the socket option IPV6_V6ONLY is always set to true. Thus, this allows
    // binding two sockets to the same port (one for each domain, ipv4 and
    // ipv6).
    if !is_ipv4 {
        socket.set_only_v6(true)?;
    }
    socket.bind(&addr)?;
    socket.listen(1024)?;
    Ok(socket.into_tcp_listener())
}

/// Create a hyper server with the provided `filter`, binding to `addr`. This also sets the
/// `TCP_NODELAY` flag on incoming connections.
pub(crate) fn serve_it<T: Into<SocketAddr>>(
    addr: T,
    filter: BoxedFilter<(impl Reply + 'static,)>,
) -> anyhow::Result<(SocketAddr, impl Future<Output = anyhow::Result<()>>)> {
    let filtered_service = warp::service(filter);

    let make_svc = make_service_fn(move |_| {
        let filtered_service = filtered_service.clone();
        async move { Ok::<_, Infallible>(filtered_service) }
    });

    let listener = make_listener(addr)?;
    let bound_to = listener.local_addr()?;
    let builder = Server::from_tcp(listener)?;
    let fut = builder.tcp_nodelay(true).serve(make_svc).map_err(|e| e.into());
    Ok((bound_to, fut))
}
