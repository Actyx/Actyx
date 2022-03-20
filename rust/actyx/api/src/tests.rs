use actyx_sdk::{app_id, service::AuthenticationResponse, NodeId};
use bytes::Bytes;
use chrono::Utc;
use crypto::{KeyStore, KeyStoreRef, PrivateKey, PublicKey};
use hyper::Response;
use parking_lot::lock_api::RwLock;
use serde_json::*;
use swarm::{
    blob_store::BlobStore,
    event_store_ref::{self, EventStoreHandler, EventStoreRef},
    BanyanStore, DbPath,
};
use warp::*;

use crate::{
    auth::create_token, files::FilePinner, formats::Licensing, rejections, util::NodeInfo, AppMode, EventService,
};
use tokio::{runtime::Handle, sync::mpsc};

const UNAUTHORIZED_TOKEN: &str = "AAAAWaZnY3JlYXRlZBsABb3ls11m8mZhcHBfaWRyY29tLmV4YW1wbGUubXktYXBwZmN5Y2xlcwBndmVyc2lvbmUxLjAuMGh2YWxpZGl0eRkBLGlldmFsX21vZGX1AQv+4BIlF/5qZFHJ7xJflyew/CnF38qdV1BZr/ge8i0mPCFqXjnrZwqACX5unUO2mJPsXruWYKIgXyUQHwKwQpzXceNzo6jcLZxvAKYA05EFDnFvPIRfoso+gBJinSWpDQ==";

const TRACE: bool = false;
static INIT: std::sync::Once = std::sync::Once::new();
pub fn initialize() {
    if TRACE {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter("debug,tracing=info,warp=debug".to_owned())
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
    let auth_args = NodeInfo {
        cycles: 0.into(),
        key_store: key_store.clone(),
        node_id: node_key.into(),
        token_validity: 300,
        ax_public_key: PrivateKey::generate().into(),
        licensing: Licensing::default(),
        started_at: Utc::now(),
    };
    let event_store = {
        let store2 = store.clone();
        let (tx, mut rx) = mpsc::channel(100);
        store.spawn_task("handler", async move {
            let mut handler = EventStoreHandler::new(store2);
            let runtime = Handle::current();
            while let Some(request) = rx.recv().await {
                handler.handle(request, &runtime);
            }
        });
        EventStoreRef::new(move |e| tx.try_send(e).map_err(event_store_ref::Error::from))
    };
    let event_service = EventService::new(event_store, auth_args.node_id);
    let pinner = FilePinner::new(event_service.clone(), store.ipfs().clone());
    let blobs = BlobStore::new(DbPath::Memory).unwrap();
    let route =
        super::routes(auth_args.clone(), store, event_service, pinner, blobs).with(warp::trace::named("api_test"));

    let token = create_token(
        auth_args,
        app_id!("com.example.my-app"),
        "1.0.0".into(),
        AppMode::Signed,
    )
    .unwrap();
    (route, token.to_string(), node_key, key_store)
}

#[track_caller]
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
        .path("/api/v2/auth")
        .method("POST")
        .json(&payload)
        .reply(&route)
        .await;

    assert_eq!(resp.status(), http::StatusCode::OK);

    let bytes = resp.body();
    let AuthenticationResponse { token, .. } = serde_json::from_slice(bytes).unwrap();

    let resp = test::request()
        .path("/api/v2/events/offsets")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn node_id() {
    let (route, _, node_key, ..) = test_routes().await;
    let resp = test::request().path("/api/v2/node/id").reply(&route).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "text/plain; charset=utf-8");
    assert_eq!(resp.body(), &NodeId::from(node_key).to_string())
}

#[tokio::test]
async fn ok() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
        .header("Authorization", format!("Bearer {}", token))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/json");

    let resp = test::request()
        .path("/api/v2/events/offsets")
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
        .json(&json!({"offsets": {}, "upperBound": {}, "query": "FROM 'a'", "order": "asc"}))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/x-ndjson");

    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/x-ndjson")
        .json(&json!({"offsets": {}, "upperBound": {}, "query": "FROM 'a'", "order": "asc"}))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/x-ndjson");
}

#[tokio::test]
async fn ok_accept_wildcard() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
        .method("OPTIONS")
        .header("Origin", "http://localhost")
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "X-Custom")
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::FORBIDDEN); // header not allowed

    let resp = test::request()
        .path("/api/v2/events/offsets")
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
        http::StatusCode::NOT_FOUND,
        json!({
          "code": "ERR_NOT_FOUND",
          "message": "The requested resource could not be found."
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
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
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
    let resp = test::request().path("/api/v2/events/offsets").reply(&route).await;
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
        .path("/api/v2/events/offsets")
        .header("Authorization", "Foo hello")
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::UNAUTHORIZED,
        json!({
          "code": "ERR_UNSUPPORTED_AUTH_TYPE",
          "message": "Unsupported authentication type 'Foo'. Only \"Bearer\" is supported."
        }),
    );
}

#[tokio::test]
async fn unauthorized_invalid() {
    let (route, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/offsets")
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
        .path("/api/v2/events/offsets")
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
async fn unsupported_media_type() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/x-ndjson")
        .header("Content-Type", "text/plain")
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
        json!({
          "code": "ERR_UNSUPPORTED_MEDIA_TYPE",
          "message": "The request's content-type is not supported."
        }),
    );
}

#[tokio::test]
async fn not_acceptable() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/offsets")
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
          "code": "ERR_BAD_REQUEST",
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
          "code": "ERR_BAD_REQUEST",
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
        .json(&json!({"offsets": null, "query": "FROM x"}))
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::BAD_REQUEST,
        json!({
          "code": "ERR_BAD_REQUEST",
          "message": "Invalid request.  --> 1:6\n  |\n1 | FROM x\n  |      ^---\n  |\n  = expected array or tag_expr"
        }),
    );
}

#[tokio::test]
async fn bad_request_unknown_stream() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
          "upperBound": {"4Rf5nier.0HWMLwRm32Nbgx8pkkOMCahfEmRtHCWaSs-0": 42},
          "query": "FROM 'x'",
          "order": "asc"
        }))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let js = serde_json::from_slice::<serde_json::Value>(&*resp.body()).unwrap();
    assert_eq!(
        js,
        json!({ "type": "diagnostic", "severity": "error", "message": "Query bounds out of range: upper bound must be within the known present."})
    );
}

#[tokio::test]
async fn bad_request_invalid_upper_bounds() {
    let (route, token, node_key, ..) = test_routes().await;
    let stream_id = NodeId::from(node_key).stream(0.into()).to_string();
    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
          "upperBound": {stream_id: 42},
          "query": "FROM 'x'",
          "order": "asc"
        }))
        .reply(&route)
        .await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let js = serde_json::from_slice::<serde_json::Value>(&*resp.body()).unwrap();
    assert_eq!(
        js,
        json!({ "type": "diagnostic", "severity": "error", "message": "Query bounds out of range: upper bound must be within the known present."})
    );
}

#[tokio::test]
async fn bad_request_aql_feature() {
    let (route, token, ..) = test_routes().await;
    let resp = test::request()
        .path("/api/v2/events/query")
        .method("POST")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({"query": "FROM from(2021-07-20Z)", "order":"asc"}))
        .reply(&route)
        .await;
    assert_err_response(
        resp,
        http::StatusCode::BAD_REQUEST,
        json!({"code": "ERR_BAD_REQUEST", "message": "Invalid request. The query uses beta features that are not enabled: timeRange."}),
    );
}

#[tokio::test]
async fn ws_aql_feature() -> anyhow::Result<()> {
    fn to_json(m: ws::Message) -> anyhow::Result<serde_json::Value> {
        Ok(m.to_str()
            .map_err(|_| anyhow::anyhow!("binary"))?
            .parse::<serde_json::Value>()?)
    }
    async fn assert_complete(ws: &mut test::WsClient, id: u32) {
        assert_eq!(
            to_json(ws.recv().await.unwrap()).unwrap(),
            json!({"type": "complete", "requestId": id})
        );
    }

    let (route, token, ..) = test_routes().await;
    let mut ws = test::ws()
        .path(&format!("/api/v2/events?{}", token))
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .handshake(route)
        .await?;

    ws.send_text(
        json!({
            "type": "request",
            "serviceId": "query",
            "requestId": 1,
            "payload": "x"
        })
        .to_string(),
    )
    .await;
    assert_eq!(
        to_json(ws.recv().await?)?,
        json!({
            "type": "error",
            "requestId": 1,
            "kind": {
                "type": "badRequest",
                "message": r#"invalid type: string "x", expected struct QueryRequest"#
            }
        })
    );
    assert_complete(&mut ws, 1).await;

    ws.send_text(
        json!({
            "type": "request",
            "serviceId": "query",
            "requestId": 1,
            "payload": {
                "query": "x",
                "order": "asc",
            }
        })
        .to_string(),
    )
    .await;
    assert_eq!(
        to_json(ws.recv().await?)?,
        json!({
            "type": "error",
            "requestId": 1,
            "kind": {
                "type": "serviceError",
                "value": "Invalid request.  --> 1:1\n  |\n1 | x\n  | ^---\n  |\n  = expected main_query"
            }
        })
    );
    assert_complete(&mut ws, 1).await;

    ws.send_text(
        json!({
            "type": "request",
            "serviceId": "query",
            "requestId": 1,
            "payload": {
                "query": "FROM from(2021-07-20Z)",
                "order": "asc",
            }
        })
        .to_string(),
    )
    .await;
    assert_eq!(
        to_json(ws.recv().await?)?,
        json!({
            "type": "error",
            "requestId": 1,
            "kind": {
                "type": "serviceError",
                "value": "The query uses beta features that are not enabled: timeRange."
            }
        })
    );
    assert_complete(&mut ws, 1).await;

    ws.send_text(
        json!({
            "type": "request",
            "serviceId": "query",
            "requestId": 1,
            "payload": {
                "query": "FEATURES(timeRange) FROM from(2021-07-20Z)",
                "order": "asc",
            }
        })
        .to_string(),
    )
    .await;
    assert_eq!(*to_json(ws.recv().await?)?.pointer("/type").unwrap(), json!("next"));

    Ok(())
}

mod files {
    use std::{collections::BTreeMap, time::Duration};

    use actyx_sdk::service::DirectoryChild;
    use maplit::btreemap;

    use super::*;

    fn create_mutlipart(files: BTreeMap<&str, Vec<u8>>) -> Vec<u8> {
        let mut buf = vec![];
        let boundary = "boundary";
        for (k, v) in files {
            buf.extend_from_slice(b"--");
            buf.extend_from_slice(boundary.as_bytes());
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(
                format!(r#"Content-Disposition: form-data; name="file"; filename="{}""#, k).as_bytes(),
            );
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
            buf.extend_from_slice(&v[..]);
            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"--");
        buf.extend_from_slice(boundary.as_bytes());
        buf.extend_from_slice(b"--\r\n");
        buf
    }

    #[tokio::test]
    async fn adding_files() -> anyhow::Result<()> {
        let (route, token, ..) = test_routes().await;
        let body = create_mutlipart(btreemap! {
            "folder/my-filename" => b"42\n".to_vec(),
            "folder/my-filename2" => b"42\n".to_vec(),
        });
        let resp = test::request()
            .path("/api/v2/files")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                r#"multipart/form-data; charset=utf-8; boundary="boundary""#,
            )
            // .body(..) also sets Content-Length
            .body(body)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let cid = String::from_utf8(resp.body().to_vec())?;
        assert_eq!("bafybeih3rdoefyjmhg2wcu34njtwjc6kz44voehswqpr2dnplqjiv3opzi", &*cid);
        Ok(())
    }

    #[tokio::test]
    async fn adding_and_retrieving_directories() -> anyhow::Result<()> {
        let (route, token, ..) = test_routes().await;
        let body = create_mutlipart(btreemap! {
            "folder/my-filename" => b"42\n".to_vec(),
            "folder/my-filename2" => b"42\n".to_vec(),
        });
        let resp = test::request()
            .path("/api/v2/files")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                r#"multipart/form-data; charset=utf-8; boundary="boundary""#,
            )
            // .body(..) also sets Content-Length
            .body(body)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let cid = String::from_utf8(resp.body().to_vec())?;

        // get json directory listing
        let resp = test::request()
            .path(&format!("/api/v2/files/{}", cid))
            .method("GET")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let listing: actyx_sdk::service::FilesGetResponse = serde_json::from_slice(resp.body())?;
        let expected = actyx_sdk::service::FilesGetResponse::Directory {
            name: "/".into(),
            cid: "bafybeih3rdoefyjmhg2wcu34njtwjc6kz44voehswqpr2dnplqjiv3opzi"
                .parse()
                .unwrap(),
            children: vec![DirectoryChild {
                size: 121,
                name: "folder".into(),
                cid: "bafybeidzcta4duz77hvyyikfd7fjhwls6pebx766hderwkgk73nwktbgaa"
                    .parse()
                    .unwrap(),
            }],
        };
        assert_eq!(listing, expected);

        // get redirect for html directory listing
        let resp = test::request()
            .path(&format!("/api/v2/files/{}", cid))
            .method("GET")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/html")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::MOVED_PERMANENTLY);
        let new_location = resp.headers().get("Location").unwrap();

        // and now get html directory listing
        let resp = test::request()
            .path(new_location.to_str()?)
            .method("GET")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/html")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert!(String::from_utf8(resp.body().to_vec())?.contains("<body>"));
        Ok(())
    }

    #[tokio::test]
    async fn adding_and_retrieving_files() -> anyhow::Result<()> {
        let (route, token, ..) = test_routes().await;
        let body = create_mutlipart(btreemap! {
            "folder/my-filename" => b"42\n".to_vec(),
            "folder/my-filename2" => b"42\n".to_vec(),
        });
        let resp = test::request()
            .path("/api/v2/files")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                r#"multipart/form-data; charset=utf-8; boundary="boundary""#,
            )
            // .body(..) also sets Content-Length
            .body(body)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK, "{:?}", resp);
        let cid = String::from_utf8(resp.body().to_vec())?;

        let resp = test::request()
            .path(&format!("/api/v2/files/{}/folder/my-filename", cid))
            .method("GET")
            .header("Authorization", format!("Bearer {}", token))
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Disposition").unwrap().to_str()?,
            r#"inline;filename="my-filename""#
        );
        assert_eq!(resp.headers().get("Content-Type").unwrap().to_str()?, "text/plain");
        assert_eq!(resp.body().to_vec(), b"42\n".to_vec());

        Ok(())
    }

    #[tokio::test]
    async fn retrieving_files_via_root() -> anyhow::Result<()> {
        let (route, token, ..) = test_routes().await;
        let body = create_mutlipart(btreemap! {
            "my-filename" => b"42\n".to_vec(),
            "index.html" => b"Hello World!\n".to_vec(),
        });
        let resp = test::request()
            .path("/api/v2/files")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                r#"multipart/form-data; charset=utf-8; boundary="boundary""#,
            )
            // .body(..) also sets Content-Length
            .body(body)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let cid = String::from_utf8(resp.body().to_vec())?;

        // get index.html, either served via the root or indexed
        for path in ["/", "/index.html"] {
            let resp = test::request()
                .path(path)
                .method("GET")
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "text/html")
                .header("Host", &format!("{}.actyx.localhost", cid))
                .reply(&route)
                .await;
            assert_eq!(resp.status(), http::StatusCode::OK);
            assert_eq!(
                resp.headers().get("Content-Disposition").unwrap().to_str()?,
                r#"inline;filename="index.html""#
            );
            assert_eq!(resp.headers().get("Content-Type").unwrap().to_str()?, "text/html");
            assert_eq!(resp.body().to_vec(), b"Hello World!\n".to_vec());

            // check w/o token
            let resp = test::request()
                .path(path)
                .method("GET")
                .header("Accept", "text/html")
                .header("Host", &format!("{}.actyx.localhost", cid))
                .reply(&route)
                .await;
            assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        }

        // set a name
        let name = "w00pw00p";
        let resp = test::request()
            .path(&format!("/api/v2/files/{}", name))
            .method("PUT")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/html")
            .body(&cid)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        // Some time to let the events round trip
        tokio::time::sleep(Duration::from_millis(50)).await;

        // get index.html, either served via the root or indexed
        // WITHOUT token served via `<name>.actyx.localhost`
        for path in ["/", "/index.html"] {
            let resp = test::request()
                .path(path)
                .method("GET")
                .header("Accept", "text/html")
                .header("Host", &format!("{}.actyx.localhost", name))
                .reply(&route)
                .await;
            assert_eq!(resp.status(), http::StatusCode::OK);
            assert_eq!(
                resp.headers().get("Content-Disposition").unwrap().to_str()?,
                r#"inline;filename="index.html""#
            );
            // responses for names must not be cached
            assert_eq!(
                resp.headers().get("Cache-Control").unwrap().to_str()?,
                "no-cache, no-store, must-revalidate"
            );
            assert_eq!(resp.headers().get("Content-Type").unwrap().to_str()?, "text/html");
            assert_eq!(resp.body().to_vec(), b"Hello World!\n".to_vec());
        }

        // delete the name
        let resp = test::request()
            .path(&format!("/api/v2/files/{}", name))
            .method("DELETE")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/html")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        // Some time to let the events round trip
        tokio::time::sleep(Duration::from_millis(50)).await;
        for path in ["/", "/index.html"] {
            let resp = test::request()
                .path(path)
                .method("GET")
                .header("Accept", "text/html")
                .header("Host", &format!("{}.actyx.localhost", name))
                .reply(&route)
                .await;
            assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        }
        Ok(())
    }

    #[tokio::test]
    async fn should_reject_cids_as_names() -> anyhow::Result<()> {
        let (route, token, ..) = test_routes().await;
        let body = create_mutlipart(btreemap! {
            "my-filename" => b"42\n".to_vec(),
            "index.html" => b"Hello World!\n".to_vec(),
        });
        let resp = test::request()
            .path("/api/v2/files")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                r#"multipart/form-data; charset=utf-8; boundary="boundary""#,
            )
            // .body(..) also sets Content-Length
            .body(body)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let cid = String::from_utf8(resp.body().to_vec())?;

        let resp = test::request()
            .path(&format!("/api/v2/files/{}", cid))
            .method("PUT")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/html")
            .body(&cid)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), http::StatusCode::METHOD_NOT_ALLOWED);

        Ok(())
    }

    #[tokio::test]
    async fn should_return_404_in_root() -> anyhow::Result<()> {
        let (route, ..) = test_routes().await;
        for base in ["/", "/I/dont/exist"] {
            let resp = test::request().path(base).method("GET").reply(&route).await;
            assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
        }

        Ok(())
    }
}
