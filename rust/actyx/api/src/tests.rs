use actyxos_sdk::{app_id, service::AuthenticationResponse};
use bytes::Bytes;
use crypto::{KeyStore, KeyStoreRef, PublicKey};
use hyper::Response;
use parking_lot::lock_api::RwLock;
use serde_json::*;
use swarm::BanyanStore;
use warp::*;

use crate::{authentication_service_api::create_token, rejections, util::AuthArgs, AppMode};

const UNAUTHORIZED_TOKEN: &str = "AAAAWaZnY3JlYXRlZBsABb3ls11m8mZhcHBfaWRyY29tLmV4YW1wbGUubXktYXBwZmN5Y2xlcwBndmVyc2lvbmUxLjAuMGh2YWxpZGl0eRkBLGlldmFsX21vZGX1AQv+4BIlF/5qZFHJ7xJflyew/CnF38qdV1BZr/ge8i0mPCFqXjnrZwqACX5unUO2mJPsXruWYKIgXyUQHwKwQpzXceNzo6jcLZxvAKYA05EFDnFvPIRfoso+gBJinSWpDQ==";

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

async fn test_routes() -> (
    impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone,
    String,
    PublicKey,
    KeyStoreRef,
) {
    initialize();
    let key_store = std::sync::Arc::new(RwLock::new(KeyStore::default()));
    let node_key = key_store.write().generate_key_pair().unwrap();
    let store = BanyanStore::test("api").await.unwrap();
    let route = super::routes(node_key.into(), store, key_store.clone(), 0.into()).with(warp::trace::named("api_test"));

    let auth_args = AuthArgs {
        cycles: 0.into(),
        key_store: key_store.clone(),
        node_key,
        token_validity: 300,
    };
    let token = create_token(
        auth_args,
        app_id!("com.example.my-app"),
        "1.0.0".into(),
        AppMode::Signed,
    )
    .unwrap();
    (route, token.to_string(), node_key, key_store)
}

fn assert_err_response(resp: Response<Bytes>, status: http::StatusCode, json: serde_json::Value) {
    assert_eq!(resp.status(), status);
    assert_eq!(serde_json::from_slice::<serde_json::Value>(resp.body()).unwrap(), json);
}

#[tokio::test]
async fn authenticate() {
    let payload = json!({
      "appId": "com.example.my-app","displayName": "My Example App","version": "1.0.0"
    });

    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/authenticate")
        .method("POST")
        .json(&payload)
        .reply(&route)
        .await;

    assert_eq!(resp.status(), http::StatusCode::OK);

    let bytes = resp.body();
    let AuthenticationResponse { token, .. } = serde_json::from_slice(bytes).unwrap();

    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn ok() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");
}

#[tokio::test]
async fn ok_accept_json() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");

    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");
}

#[tokio::test]
async fn ok_accept_ndjson() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({"offsets": {}, "upperBound": {}, "where": "'a'", "order": "asc"}))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/x-ndjson");

    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/x-ndjson")
        .json(&json!({"offsets": {}, "upperBound": {}, "where": "'a'", "order": "asc"}))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/x-ndjson");
}

#[tokio::test]
async fn ok_accept_star() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "*/*")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");
}

#[tokio::test]
async fn ok_accept_multiple() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json, text/plain, */*") // this is what NodeJS sends
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");
}

#[tokio::test]
async fn ok_cors() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .method("OPTIONS")
        .header("Origin", "http://localhost")
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "Authorization, Accept, Content-Type")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn forbidden_cors() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .method("OPTIONS")
        .header("Origin", "http://localhost")
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "X-Custom")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::FORBIDDEN); // header not allowed

    let resp = test::request()
        .path("/api/v2/events/node_id")
        .method("OPTIONS")
        .header("Origin", "http://localhost")
        .header("Access-Control-Request-Method", "XXX")
        .header("Access-Control-Request-Headers", "Accept")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::FORBIDDEN); // method not allowed
}

#[tokio::test]
async fn ws() {
    let (route, token, ..) = test_routes().await;

    let ws_test = |path: &str| {
        test::ws()
            .path(&format!("/api/v2/events{}?{}", path, token.clone()))
            .handshake(route.clone())
    };
    assert!(ws_test("").await.is_ok());
    assert!(ws_test("/").await.is_ok());

    let ws_test = |path: &str| {
        test::request()
            .path(&format!("/api/v2/events{}", path))
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .reply(&route)
    };

    assert_err_response(
        ws_test("").await,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_MISSING_TOKEN_PARAM",
          "message": "\"token\" parameter is missing."
        }),
    );

    assert_err_response(
        ws_test(&format!("?{}", UNAUTHORIZED_TOKEN)).await,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_TOKEN_UNAUTHORIZED",
          "message": "Unauthorized token."
        }),
    );

    assert_err_response(
        ws_test(&format!("/x?{}", token)).await,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_MISSING_AUTH_HEADER",
          "message": "\"Authorization\" header is missing."
        }),
    );
}

#[tokio::test]
async fn internal_err() {
    // Simulate internal err
    let route = any()
        .and_then(|| async move { Err::<String, _>(reject::custom(rejections::Crash)) })
        .recover(|r| async { rejections::handle_rejection(r) });
    let resp = test::request().reply(&route).await;
    assert_err_response(
        resp,
        http::StatusCode::INTERNAL_SERVER_ERROR,
        json!({
          "code": "ERR_INTERNAL",
          "message": "Internal server error."
        }),
    );
}

#[tokio::test]
async fn unauthorized() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", UNAUTHORIZED_TOKEN))
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_TOKEN_UNAUTHORIZED",
          "message": "Unauthorized token."
        }),
    );
}

#[tokio::test]
async fn should_fail_when_token_payload_shape_is_wrong() {
    let (route, _, node_key, key_store) = test_routes().await;
    let bytes = serde_cbor::to_vec(&"1,2,3".to_string()).unwrap();
    let signed = key_store.read().sign(bytes, vec![node_key]).unwrap();
    let token_with_wrong_payload = base64::encode(signed);

    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token_with_wrong_payload))
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::BAD_REQUEST,
        json!({
          "code": "ERR_TOKEN_INVALID",
          "message": format!("Invalid token: \'{}\'. Cannot parse CBOR. Please provide a valid bearer token.", token_with_wrong_payload)
        }),
    );
}

#[tokio::test]
async fn unauthorized_missing_header() {
    let (route, ..) = test_routes().await;
    let resp = test::request().path("/api/v2/events/node_id").reply(&route).await;
    assert_err_response(
        resp,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_MISSING_AUTH_HEADER",
          "message": "\"Authorization\" header is missing."
        }),
    );
}

#[tokio::test]
async fn unauthorized_unsupported() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", "Foo hello")
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_WRONG_AUTH_TYPE",
          "message": "Unsupported authentication type 'Foo'. Only \"Bearer\" is supported."
        }),
    );
}

#[tokio::test]
async fn unauthorized_invalid() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", "Bearer invalid")
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::BAD_REQUEST,
        json!({
          "code": "ERR_TOKEN_INVALID",
          "message": "Invalid token: 'invalid'. Cannot parse token bytes. Please provide a valid bearer token."
        }),
    );
}

#[tokio::test]
async fn not_found() {
    let (route, ..) = test_routes().await;
    let resp = test::request().path("/nowhere").reply(&route).await;
    assert_err_response(
        resp,
        http::StatusCode::NOT_FOUND,
        json!({
          "code": "ERR_NOT_FOUND",
          "message": "The requested resource could not be found."
        }),
    );
}

#[tokio::test]
async fn method_not_allowed() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
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
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/node_id")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "text/html")
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::NOT_ACCEPTABLE,
        json!({
          "code": "ERR_NOT_ACCEPTABLE",
          "message": "Content with type 'text/html' was requested but the resource is only capable of generating content of the following type(s): */*, application/json."
        }),
    );
}

#[tokio::test]
async fn bad_request_invalid_json() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/publish")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .body("Jason vs. Freddy")
        .reply(&route)
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
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/publish")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .body("{}")
        .reply(&route)
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
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/subscribe")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({"offsets": null, "where": "here"}))
        .reply(&route)
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
