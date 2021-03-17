mod event_service_api;
mod ipfs_file_gateway;
mod public_api;
mod util;

use std::net::SocketAddr;

use actyxos_sdk::NodeId;
use crypto::KeyStoreRef;
use futures::future::try_join_all;
use swarm::BanyanStore;
use warp::*;

use crate::util::hyper_serve::serve_it;

pub async fn run(
    node_id: NodeId,
    store: BanyanStore,
    bind_to: impl Iterator<Item = SocketAddr> + Send,
    key_store: KeyStoreRef,
) {
    let api = routes(node_id, store, key_store);
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
    node_id: NodeId,
    store: BanyanStore,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let event_service = event_service_api::service::EventService::new(store.clone());
    let event_service_api = event_service_api::routes(node_id, event_service, key_store);

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
    use crate::util::rejections::*;
    if let Some(reject::MethodNotAllowed { .. }) = r.find() {
        Ok((http::StatusCode::METHOD_NOT_ALLOWED, ApiError::MethodNotAllowed))
    } else if let Some(NotAcceptable { requested, supported }) = r.find() {
        Ok((
            http::StatusCode::NOT_ACCEPTABLE,
            ApiError::NotAcceptable {
                requested: requested.to_owned(),
                supported: supported.to_owned(),
            },
        ))
    } else if let Some(e) = r.find::<filters::body::BodyDeserializeError>() {
        use std::error::Error;
        Ok((
            http::StatusCode::BAD_REQUEST,
            ApiError::BadRequest {
                cause: e.source().map_or("".to_string(), |e| e.to_string()),
            },
        ))
    } else if let Some(Unauthorized::MissingToken(source)) = r.find() {
        Ok((
            http::StatusCode::UNAUTHORIZED,
            ApiError::MissingAuthToken {
                source: source.to_owned(),
            },
        ))
    } else if let Some(Unauthorized::TokenUnauthorized) = r.find() {
        Ok((http::StatusCode::UNAUTHORIZED, ApiError::TokenUnauthorized))
    } else if let Some(Unauthorized::UnsupportedAuthType(auth_type)) = r.find() {
        Ok((
            http::StatusCode::UNAUTHORIZED,
            ApiError::UnsupportedAuthType {
                requested: auth_type.to_owned(),
            },
        ))
    } else if let Some(Unauthorized::InvalidBearerToken(token)) = r.find() {
        Ok((
            http::StatusCode::UNAUTHORIZED,
            ApiError::TokenInvalid {
                token: token.to_owned(),
            },
        ))
    } else {
        println!("unhandled rejection: {:?}", r);
        Err(r)
    }
    .map(|(status, code)| {
        let api_error: ApiErrorResponse = code.into();
        let json = warp::reply::json(&api_error);
        warp::reply::with_status(json, status)
    })
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use crypto::KeyStore;
    use hyper::Response;
    use parking_lot::lock_api::RwLock;
    use serde_json::*;
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
        super::routes(store.node_id(), store, key_store).with(warp::trace::named("api_test"))
    }

    fn assert_err_response(resp: Response<Bytes>, status: http::StatusCode, json: serde_json::Value) {
        assert_eq!(resp.status(), status);
        assert_eq!(serde_json::from_slice::<serde_json::Value>(resp.body()).unwrap(), json);
    }

    #[tokio::test]
    async fn ok() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("Authorization", "Bearer ok")
            .reply(&test_routes().await)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ok_accept() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("Authorization", "Bearer ok")
            .header("accept", "application/json")
            .reply(&test_routes().await)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn ws() {
        assert!(test::ws()
            .path("/api/v2/events?token=ok")
            .handshake(test_routes().await)
            .await
            .is_ok());
        assert!(test::ws()
            .path("/api/v2/events/?token=ok")
            .handshake(test_routes().await)
            .await
            .is_ok());

        assert!(test::ws()
            .path("/api/v2/events?token=disallow")
            .handshake(test_routes().await)
            .await
            .is_err());
        assert!(test::ws()
            .path("/api/v2/events/x?token=ok")
            .handshake(test_routes().await)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn unauthorized() {
        let resp = test::request()
            .path("/api/v2/events/node_id?")
            .header("Authorization", "Bearer disallow")
            .reply(&test_routes().await)
            .await;
        assert_err_response(
            resp,
            http::StatusCode::UNAUTHORIZED,
            json!({
              "code": "ERR_TOKEN_UNAUTHORIZED",
              "message": "Authorization request header contains an unauthorized token."
            }),
        );
    }

    #[tokio::test]
    async fn unauthorized_missing_token() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .reply(&test_routes().await)
            .await;
        assert_err_response(
            resp,
            http::StatusCode::UNAUTHORIZED,
            json!({
              "code": "ERR_EMPTY_AUTH_HEADER",
              "message": "Authorization token is missing. Please provide a valid auth token header."
            }),
        );
    }

    #[tokio::test]
    async fn unauthorized_unsupported() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("Authorization", "Foo hello")
            .reply(&test_routes().await)
            .await;
        assert_err_response(
            resp,
            http::StatusCode::UNAUTHORIZED,
            json!({
              "code": "ERR_WRONG_AUTH_TYPE",
              "message": "Unsupported Authorization header type 'Foo'. Please provide a Bearer token."
            }),
        );
    }

    #[tokio::test]
    async fn unauthorized_invalid() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("Authorization", "Bearer invalid")
            .reply(&test_routes().await)
            .await;
        assert_err_response(
            resp,
            http::StatusCode::UNAUTHORIZED,
            json!({
              "code": "ERR_TOKEN_INVALID",
              "message": "Invalid token: 'invalid'. Please provide a valid Bearer token."
            }),
        );
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
            .header("Authorization", "Bearer ok")
            .reply(&(test_routes().await))
            .await;
        assert_err_response(
            resp,
            http::StatusCode::METHOD_NOT_ALLOWED,
            json!({
              "code": "ERR_METHOD_NOT_ALLOWED",
              "message": "Method not supported."
            }),
        );
    }

    #[tokio::test]
    async fn not_acceptable() {
        let resp = test::request()
            .path("/api/v2/events/node_id")
            .header("Authorization", "Bearer ok")
            .header("accept", "text/html")
            .reply(&(test_routes().await))
            .await;
        assert_err_response(
            resp,
            http::StatusCode::NOT_ACCEPTABLE,
            json!({
              "code": "ERR_NOT_ACCEPTABLE",
              "message": "The requested resource is only capable of generating content of type 'application/json' but 'text/html' was requested."
            }),
        );
    }

    #[tokio::test]
    async fn bad_request_invalid_json() {
        let resp = test::request()
            .path("/api/v2/events/publish")
            .method("POST")
            .header("Authorization", "Bearer ok")
            .body("Jason vs. Freddy")
            .reply(&(test_routes().await))
            .await;
        assert_err_response(
            resp,
            http::StatusCode::BAD_REQUEST,
            json!({
              "code": "ERR_MALFORMED_REQUEST_SYNTAX",
              "message": "Invalid request. expected value at line 1 column 1"
            }),
        );
    }

    #[tokio::test]
    async fn bad_request_invalid_request() {
        let resp = test::request()
            .path("/api/v2/events/publish")
            .method("POST")
            .header("Authorization", "Bearer ok")
            .body("{}")
            .reply(&(test_routes().await))
            .await;
        assert_err_response(
            resp,
            http::StatusCode::BAD_REQUEST,
            json!({
              "code": "ERR_MALFORMED_REQUEST_SYNTAX",
              "message": "Invalid request. missing field `data` at line 1 column 2"
            }),
        );
    }

    #[tokio::test]
    async fn bad_request_invalid_expression() {
        let resp = test::request()
            .path("/api/v2/events/subscribe")
            .method("POST")
            .header("Authorization", "Bearer ok")
            .json(&serde_json::json!({"offsets": null, "where": "here"}))
            .reply(&(test_routes().await))
            .await;
        assert_err_response(
            resp,
            http::StatusCode::BAD_REQUEST,
            json!({
              "code": "ERR_MALFORMED_REQUEST_SYNTAX",
              "message": "Invalid request. 0: at line 1:\nhere\n^\nexpected \'\'\', found h\n\n1: at line 1, in literal:\nhere\n^\n\n2: at line 1, in Alt:\nhere\n^\n\n3: at line 1, in and:\nhere\n^\n\n4: at line 1, in or:\nhere\n^\n\n at line 1 column 31"
            }),
        );
    }
}
