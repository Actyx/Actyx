mod validate_signed_manifest;

use ax_types::{types::Binary, AppId, AppManifest, Timestamp};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use warp::{body, post, reply, Filter, Rejection, Reply};

use crate::{
    api::{
        bearer_token::BearerToken, filters::accept_json, licensing::Licensing, reject, rejections::ApiError, AppMode,
        NodeInfo, Token,
    },
    crypto::{PublicKey, SignedMessage},
};

use validate_signed_manifest::validate_signed_manifest;

fn mk_success_log_msg(token: &BearerToken) -> String {
    let expiration_time: DateTime<Utc> = token.expiration().try_into().expect("generated timestamp");
    let mode = match token.app_mode {
        AppMode::Trial => "trial",
        AppMode::Signed => "production",
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
    tracing::info!(target: "AUTH", "{}", mk_success_log_msg(&token));
    Ok(base64::encode(signed).into())
}

pub(crate) fn verify_token(node_info: NodeInfo, token: Token) -> Result<BearerToken, ApiError> {
    let token = token.to_string();
    let bin: Binary = token.parse().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Cannot parse token bytes.".to_owned(),
    })?;
    let signed_msg: SignedMessage = bin.as_ref().try_into().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Not a signed token.".to_owned(),
    })?;
    node_info
        .key_store
        .read()
        .verify(&signed_msg, vec![node_info.node_id.into()])
        .map_err(|_| ApiError::TokenUnauthorized)?;
    let bearer_token =
        serde_cbor::from_slice::<BearerToken>(signed_msg.message()).map_err(|_| ApiError::TokenInvalid {
            token: token.clone(),
            msg: "Cannot parse CBOR.".to_owned(),
        })?;
    match bearer_token.cycles != node_info.cycles || bearer_token.is_expired() {
        true => Err(ApiError::TokenExpired),
        false => Ok(bearer_token),
    }
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

fn validate_manifest(
    manifest: &AppManifest,
    ax_public_key: &PublicKey,
    licensing: &Licensing,
) -> Result<(AppMode, AppId, String), ApiError> {
    if manifest.is_signed() {
        validate_signed_manifest(manifest, ax_public_key, licensing)
            .map(|_| (AppMode::Signed, manifest.app_id(), manifest.version().to_owned()))
    } else {
        Ok((AppMode::Trial, manifest.app_id(), manifest.version().to_owned()))
    }
}

async fn handle_auth(node_info: NodeInfo, manifest: AppManifest) -> Result<impl Reply, Rejection> {
    match validate_manifest(&manifest, &node_info.ax_public_key, &node_info.licensing) {
        Ok((is_trial, app_id, version)) => create_token(node_info, app_id, version, is_trial)
            .map(|token| reply::json(&TokenResponse::new(token)))
            .map_err(reject),
        Err(x) => Err(warp::reject::custom(x)),
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
    use crate::crypto::{KeyStore, PrivateKey, PublicKey};
    use ax_types::{app_id, AppManifest};
    use chrono::Utc;
    use hyper::http;
    use parking_lot::lock_api::RwLock;
    use std::sync::Arc;
    use warp::{reject::MethodNotAllowed, test, Filter, Rejection, Reply};

    use super::{route, validate_manifest, verify_token, AppMode, NodeInfo, TokenResponse};
    use crate::api::{licensing::Licensing, rejections::ApiError};

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
            licensing: Licensing::default(),
            started_at: Utc::now(),
        };
        route(auth_args)
    }

    struct TestFixture {
        ax_public_key: PublicKey,
        trial_manifest: AppManifest,
    }

    fn setup() -> TestFixture {
        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let trial_manifest = AppManifest::trial(
            app_id!("com.example.sample"),
            "display name".to_string(),
            "version".to_string(),
        )
        .unwrap();
        TestFixture {
            ax_public_key: ax_private_key.into(),
            trial_manifest,
        }
    }

    #[tokio::test]
    async fn auth_ok() {
        let mut key_store = KeyStore::default();
        let node_key = key_store.generate_key_pair().unwrap();
        let key_store = Arc::new(RwLock::new(key_store));
        let manifest = AppManifest::trial(
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
            licensing: Licensing::default(),
            started_at: Utc::now(),
        };

        let resp = test::request()
            .method("POST")
            .json(&manifest)
            .reply(&route(auth_args.clone()))
            .await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");

        let token: TokenResponse = serde_json::from_slice(resp.body()).unwrap();
        assert!(verify_token(auth_args, token.token.into()).is_ok())
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
        let result = validate_manifest(&x.trial_manifest, &x.ax_public_key, &Licensing::default()).unwrap();
        assert_eq!(
            result,
            (
                AppMode::Trial,
                x.trial_manifest.app_id(),
                x.trial_manifest.version().to_owned()
            )
        );
    }
}
