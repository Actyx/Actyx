mod event_service_api;
mod ipfs_file_gateway;
mod public_api;
mod util;

use std::net::SocketAddr;

use crypto::KeyStoreRef;
use futures::future::try_join_all;
use swarm::BanyanStore;
use warp::*;

use crate::util::{hyper_serve::serve_it, rejections};

pub async fn run(store: BanyanStore, bind_to: impl Iterator<Item = SocketAddr> + Send, key_store: KeyStoreRef) {
    let api = routes(store, key_store);
    let tasks = bind_to
        .into_iter()
        .map(|i| {
            let (addr, task) = serve_it(i, api.clone().boxed()).unwrap();
            tracing::info!(target: "API_BOUND", "API bound to {}.", addr);
            task
        })
        .collect::<Vec<_>>();
    try_join_all(tasks).await.unwrap();
}

fn routes(
    store: BanyanStore,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let event_service = event_service_api::service::EventService::new(store.clone());
    let event_service_api = event_service_api::routes(event_service, key_store);

    let ipfs_file_gw = ipfs_file_gateway::route(store);

    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST]);

    path("ipfs")
        .and(ipfs_file_gw)
        // Note: event_service_api has a explicit rejection handler, which also
        // returns 404 no route matched. Thus it needs to come last. This should
        // eventually be refactored as part of Event Service v2.
        .or(path("api").and(path("v2").and(path("events")).and(event_service_api)))
        .recover(|r| async { handle_rejection(r) })
        .with(cors)
}

fn handle_rejection(r: Rejection) -> Result<impl Reply, Rejection> {
    match r.find() {
        Some(rejections::NotAcceptable { requested, supported }) => Ok(reply::with_status(
            format!(
                "The requested resource is only capable of generating content of type '{}' but '{}' was requested.",
                supported, requested
            ),
            http::StatusCode::NOT_ACCEPTABLE,
        )),
        _ => Err(r),
    }
}

#[cfg(test)]
mod test {
    use crypto::KeyStore;
    use parking_lot::lock_api::RwLock;
    use swarm::{BanyanStore, StoreConfig};
    use warp::*;

    const TRACE: bool = false;
    static INIT: std::sync::Once = std::sync::Once::new();
    pub fn initialize() {
        if TRACE {
            INIT.call_once(|| {
                tracing_subscriber::fmt()
                    .with_env_filter("tracing=info,warp=debug".to_owned())
                    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                    .init();
            });
        }
    }

    async fn test_routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        initialize();
        let config = StoreConfig::new("event-service-api-test".to_string());
        let store = BanyanStore::from_axconfig(config.clone()).await.unwrap();
        let key_store = std::sync::Arc::new(RwLock::new(KeyStore::default()));
        super::routes(store, key_store).with(warp::trace::named("api_test"))
    }

    #[tokio::test]
    async fn ok() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .reply(&test_routes().await)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ok_accept() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("accept", "application/json")
            .reply(&test_routes().await)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ws() {
        assert!(test::ws()
            .path("/api/v2/events")
            .handshake(test_routes().await)
            .await
            .is_ok());
        assert!(test::ws()
            .path("/api/v2/events/")
            .handshake(test_routes().await)
            .await
            .is_ok());
        assert!(test::ws()
            .path("/api/v2/events/x")
            .handshake(test_routes().await)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn not_found() {
        let resp = test::request().path("/nowhere").reply(&(test_routes().await)).await;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn method_not_allowed() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .method("POST")
            .reply(&(test_routes().await))
            .await;
        assert_eq!(resp.status(), http::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn not_acceptable() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("accept", "text/html")
            .reply(&(test_routes().await))
            .await;
        assert_eq!(
          resp.body(),
          "The requested resource is only capable of generating content of type 'application/json' but 'text/html' was requested."
        );
        assert_eq!(resp.status(), http::StatusCode::NOT_ACCEPTABLE);
    }

    #[tokio::test]
    async fn bad_request_invalid_json() {
        let filter = test_routes().await;
        let resp = test::request()
            .path("/api/v2/events/publish")
            .method("POST")
            .body("me no json")
            .reply(&filter)
            .await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.body(),
            "Request body deserialize error: expected value at line 1 column 1"
        );
    }

    #[tokio::test]
    async fn bad_request_invalid_request() {
        let filter = test_routes().await;
        let resp = test::request()
            .path("/api/v2/events/publish")
            .method("POST")
            .body("{}")
            .reply(&filter)
            .await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.body(),
            "Request body deserialize error: missing field `data` at line 1 column 2"
        );
    }

    #[tokio::test]
    async fn bad_request_invalid_expression() {
        let filter = test_routes().await;
        let resp = test::request()
            .path("/api/v2/events/subscribe")
            .method("POST")
            .json(&serde_json::json!({"offsets": null, "where": "here"}))
            .reply(&filter)
            .await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
