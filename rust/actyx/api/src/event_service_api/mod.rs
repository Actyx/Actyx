use actyxos_sdk::service::EventService;
use crypto::KeyStoreRef;
use warp::*;

mod http;
pub mod service;
mod ws;

pub fn routes<S: EventService + Clone + Send + Sync + 'static>(
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(event_service.clone(), key_store.clone()).or(ws::routes(event_service, key_store))
}

#[cfg(test)]
mod test {
    use crypto::KeyStore;
    use parking_lot::lock_api::RwLock;
    use swarm::{BanyanStore, StoreConfig};
    use warp::*;

    use super::{routes, service::EventService};

    const TRACE: bool = false;
    static INIT: std::sync::Once = std::sync::Once::new();
    pub fn initialize() {
        if TRACE {
            INIT.call_once(|| {
                let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "tracing=info,warp=debug".to_owned());
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
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
        let event_service = EventService::new(store);
        routes(event_service, key_store).with(warp::trace::named("api_test"))
    }

    #[tokio::test]
    async fn ok() {
        let resp = test::request().path("/node_id").reply(&test_routes().await).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ok_accept() {
        let resp = test::request()
            .path("/node_id")
            .header("accept", "application/json")
            .reply(&test_routes().await)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ws() {
        assert!(test::ws().path("/").handshake(test_routes().await).await.is_ok());
        assert!(test::ws().path("/x").handshake(test_routes().await).await.is_err());
    }

    #[tokio::test]
    async fn not_found() {
        let resp = test::request().path("/nowhere").reply(&(test_routes().await)).await;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn method_not_allowed() {
        let resp = test::request()
            .path("/node_id")
            .method("POST")
            .reply(&(test_routes().await))
            .await;
        assert_eq!(resp.status(), http::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn not_acceptable() {
        let resp = test::request()
            .path("/node_id")
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
            .path("/publish")
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
            .path("/publish")
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
            .path("/subscribe")
            .method("POST")
            .json(&serde_json::json!({"offsets": null, "where": "here"}))
            .reply(&filter)
            .await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
