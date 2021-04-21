use actyxos_sdk::{AppId, AppManifest, Timestamp};
use chrono::{DateTime, Utc};
use crypto::{KeyStoreRef, PublicKey};
use serde::{Deserialize, Serialize};
use tracing::*;
use warp::*;

use crate::{
    rejections::ApiError,
    util::{filters::accept_json, reject, Token},
    AppMode, BearerToken,
};

fn mk_success_log_msg(token: BearerToken) -> String {
    let expiration_time: DateTime<Utc> = token.expiration().into();
    let mode = match token.app_mode {
        AppMode::Trial => "trial",
        // TODO: replace <testing|production> with the right token when we have it
        AppMode::Signed => "<testing|production>",
    };
    format!(
        "Successfully authenticated and authorized {} for {} usage (auth token expires {})",
        token.app_id, mode, expiration_time
    )
}

pub(crate) fn create_token(
    key: PublicKey,
    key_store: KeyStoreRef,
    app_id: AppId,
    app_version: String,
    app_mode: AppMode,
    validity: u32,
) -> anyhow::Result<Token> {
    let token = BearerToken {
        created: Timestamp::now(),
        app_id,
        // TODO: to be implemented at some later point in time
        cycles: 0,
        app_version,
        validity,
        app_mode,
    };
    let bytes = serde_cbor::to_vec(&token)?;
    let signed = key_store.read().sign(bytes, vec![key])?;
    let log_msg = mk_success_log_msg(token);
    info!(target: "AUTH", "{}", log_msg);
    Ok(base64::encode(signed).into())
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenResponse {
    token: String,
}

impl TokenResponse {
    fn new(token: Token) -> Self {
        Self {
            token: token.to_string(),
        }
    }
}

fn validate_manifest(manifest: AppManifest) -> Result<AppMode, ApiError> {
    match (manifest.app_id.starts_with("com.example."), manifest.signature) {
        (true, None) => Ok(AppMode::Trial),
        // TODO: check manifest's signature
        (false, Some(_)) => Ok(AppMode::Signed),
        _ => Err(ApiError::InvalidManifest),
    }
}

async fn handle_auth(
    node_key: PublicKey,
    key_store: KeyStoreRef,
    manifest: AppManifest,
    token_validity: u32,
) -> Result<impl Reply, Rejection> {
    match validate_manifest(manifest.clone()) {
        Ok(is_trial) => create_token(
            node_key,
            key_store,
            manifest.app_id,
            manifest.version,
            is_trial,
            token_validity,
        )
        .map(|token| reply::json(&TokenResponse::new(token)))
        .map_err(reject),
        Err(x) => Err(reject::custom(x)),
    }
}

pub fn route(
    node_key: PublicKey,
    key_store: KeyStoreRef,
    token_validity: u32,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    post()
        .and(accept_json())
        .and(body::json())
        .and_then(move |manifest: AppManifest| handle_auth(node_key, key_store.clone(), manifest, token_validity))
}

#[cfg(test)]
mod tests {
    use actyxos_sdk::{app_id, AppManifest};
    use crypto::KeyStore;
    use hyper::http;
    use parking_lot::lock_api::RwLock;
    use std::sync::Arc;
    use warp::{reject::MethodNotAllowed, test, Filter, Rejection, Reply};

    use crate::{rejections::ApiError, util::filters::verify};

    use super::{route, validate_manifest, AppMode, TokenResponse};

    fn test_route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let mut key_store = KeyStore::default();
        let node_key = key_store.generate_key_pair().unwrap();
        let key_store = Arc::new(RwLock::new(key_store));
        route(node_key, key_store, 300)
    }

    #[tokio::test]
    async fn auth_ok() {
        let mut key_store = KeyStore::default();
        let node_key = key_store.generate_key_pair().unwrap();
        let key_store = Arc::new(RwLock::new(key_store));
        let manifest = AppManifest::new(
            app_id!("com.example.my-app"),
            "display name".to_string(),
            "1.0.0".to_string(),
            None,
        );

        let resp = test::request()
            .method("POST")
            .json(&manifest)
            .reply(&route(node_key, key_store.clone(), 300))
            .await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");

        let token: TokenResponse = serde_json::from_slice(resp.body()).unwrap();
        assert!(verify(token.token.into(), key_store, node_key).is_ok())
    }

    #[tokio::test]
    async fn method_not_allowed() {
        let rejection = test::request().filter(&test_route()).await.map(|_| ()).unwrap_err();
        assert!(rejection.find::<MethodNotAllowed>().is_some());
    }

    #[tokio::test]
    async fn not_acceptable() {
        let rejection = test::request()
            .method("POST")
            .header("accept", "text/html")
            .filter(&test_route())
            .await
            .map(|_| ())
            .unwrap_err();
        assert!(matches!(
            rejection.find::<ApiError>().unwrap(),
            ApiError::NotAcceptable { supported, .. } if supported == "*/*, application/json"
        ));
    }

    #[test]
    fn validate_manifest_fn() {
        let manifest = AppManifest {
            app_id: app_id!("app id"),
            display_name: "display name".to_string(),
            version: "version".to_string(),
            signature: Some("signature".to_string()),
        };

        let result = validate_manifest(manifest.clone()).unwrap();
        assert_eq!(result, AppMode::Signed);

        let ex_app_id = app_id!("com.example.");
        let result = validate_manifest(AppManifest {
            app_id: ex_app_id.clone(),
            signature: None,
            ..manifest.clone()
        })
        .unwrap();
        assert_eq!(result, AppMode::Trial);

        let result = validate_manifest(AppManifest {
            app_id: ex_app_id,
            ..manifest.clone()
        })
        .unwrap_err();
        assert!(
            matches!(result, ApiError::InvalidManifest),
            "should fail when app_id == com.example.* and sig == Some(x)"
        );

        let result = validate_manifest(AppManifest {
            signature: None,
            ..manifest
        })
        .unwrap_err();
        assert!(
            matches!(result, ApiError::InvalidManifest),
            "should fail when app_id != com.example.* and sig == None"
        );
    }
}
