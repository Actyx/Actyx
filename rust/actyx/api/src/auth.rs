use actyx_sdk::{AppId, Timestamp};
use certs::AppManifest;
use chrono::{DateTime, Utc};
use crypto::PublicKey;
use serde::{Deserialize, Serialize};
use tracing::*;
use warp::*;

use crate::{
    rejections::ApiError,
    util::{filters::accept_json, reject, NodeInfo, Token},
    AppMode, BearerToken,
};

fn mk_success_log_msg(token: &BearerToken) -> String {
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
    node_info: NodeInfo,
    app_id: AppId,
    app_version: String,
    app_mode: AppMode,
) -> anyhow::Result<Token> {
    let token = BearerToken {
        created: Timestamp::now(),
        app_id,
        cycles: node_info.cycles,
        app_version,
        validity: node_info.token_validity,
        app_mode,
    };
    let bytes = serde_cbor::to_vec(&token)?;
    let signed = node_info.key_store.read().sign(bytes, vec![node_info.node_id.into()])?;
    info!(target: "AUTH", "{}", mk_success_log_msg(&token));
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

fn validate_manifest(manifest: AppManifest, ax_public_key: PublicKey) -> Result<(AppMode, AppId, String), ApiError> {
    match manifest {
        AppManifest::Signed(x) => match x.validate(ax_public_key) {
            Ok(()) => Ok((AppMode::Signed, x.get_app_id(), x.version)),
            Err(x) => Err(ApiError::InvalidManifest { msg: x.to_string() }),
        },
        AppManifest::Trial(x) => Ok((AppMode::Trial, x.get_app_id(), x.version)),
    }
}

async fn handle_auth(node_info: NodeInfo, manifest: AppManifest) -> Result<impl Reply, Rejection> {
    match validate_manifest(manifest, node_info.ax_public_key) {
        Ok((is_trial, app_id, version)) => create_token(node_info, app_id, version, is_trial)
            .map(|token| reply::json(&TokenResponse::new(token)))
            .map_err(reject),
        Err(x) => Err(reject::custom(x)),
    }
}

pub(crate) fn route(node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    post()
        .and(accept_json())
        .and(body::json())
        .and_then(move |manifest: AppManifest| handle_auth(node_info.clone(), manifest))
}

#[cfg(test)]
mod tests {
    use actyx_sdk::app_id;
    use certs::{AppManifest, SignedAppManifest, TrialAppManifest};
    use crypto::{KeyStore, PrivateKey, PublicKey};
    use hyper::http;
    use parking_lot::lock_api::RwLock;
    use std::sync::Arc;
    use warp::{reject::MethodNotAllowed, test, Filter, Rejection, Reply};

    use super::{route, validate_manifest, AppMode, NodeInfo, TokenResponse};
    use crate::{rejections::ApiError, util::filters::verify};

    fn test_route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let mut key_store = KeyStore::default();
        let node_key = key_store.generate_key_pair().unwrap();
        let key_store = Arc::new(RwLock::new(key_store));
        let auth_args = NodeInfo {
            cycles: 0.into(),
            key_store,
            node_id: node_key.into(),
            token_validity: 300,
            ax_public_key: PrivateKey::generate().into(),
        };
        route(auth_args)
    }

    struct TestFixture {
        ax_public_key: PublicKey,
        trial_manifest: TrialAppManifest,
        signed_manifest: SignedAppManifest,
    }

    fn setup() -> TestFixture {
        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let serialized_manifest = serde_json::json!({
            "appId": "com.actyx.auth-test",
            "displayName": "auth test app",
            "version": "v0.0.1",
            "signature": "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w=="
        });
        let trial_manifest = TrialAppManifest::new(
            app_id!("com.example.sample"),
            "display name".to_string(),
            "version".to_string(),
        )
        .unwrap();
        TestFixture {
            ax_public_key: ax_private_key.into(),
            trial_manifest,
            signed_manifest: serde_json::from_value(serialized_manifest).unwrap(),
        }
    }

    #[tokio::test]
    async fn auth_ok() {
        let mut key_store = KeyStore::default();
        let node_key = key_store.generate_key_pair().unwrap();
        let key_store = Arc::new(RwLock::new(key_store));
        let manifest = TrialAppManifest::new(
            app_id!("com.example.my-app"),
            "display name".to_string(),
            "1.0.0".to_string(),
        )
        .unwrap();
        let auth_args = NodeInfo {
            cycles: 0.into(),
            key_store: key_store.clone(),
            node_id: node_key.into(),
            token_validity: 300,
            ax_public_key: PrivateKey::generate().into(),
        };

        let resp = test::request()
            .method("POST")
            .json(&manifest)
            .reply(&route(auth_args.clone()))
            .await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");

        let token: TokenResponse = serde_json::from_slice(resp.body()).unwrap();
        assert!(verify(auth_args, token.token.into()).is_ok())
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
    fn validate_manifest_should_succeed_for_trial() {
        let x = setup();
        let result = validate_manifest(AppManifest::Trial(x.trial_manifest.clone()), x.ax_public_key).unwrap();
        assert_eq!(
            result,
            (AppMode::Trial, x.trial_manifest.get_app_id(), x.trial_manifest.version)
        );
    }

    #[test]
    fn validate_manifest_should_succeed_for_signed() {
        let x = setup();
        let result = validate_manifest(AppManifest::Signed(x.signed_manifest.clone()), x.ax_public_key).unwrap();
        let expected = (
            AppMode::Signed,
            x.signed_manifest.get_app_id(),
            x.signed_manifest.version,
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn validate_manifest_should_fail_for_signed_when_ax_public_key_is_wrong() {
        let x = setup();
        let result =
            validate_manifest(AppManifest::Signed(x.signed_manifest), PrivateKey::generate().into()).unwrap_err();
        assert_eq!(
            result.to_string(),
            "Invalid manifest. Failed to validate developer certificate. Invalid signature for provided input."
        );
    }
}
