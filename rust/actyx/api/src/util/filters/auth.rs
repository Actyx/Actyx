use actyxos_sdk::{types::Binary, AppId};
use crypto::{KeyStoreRef, PublicKey};
use std::convert::TryInto;
use trees::BearerToken;
use warp::{reject, Filter, Rejection};

use crate::util::Token;
use crate::util::{
    rejections::{TokenSource, Unauthorized},
    Params,
};

fn verify(token: Token, store: KeyStoreRef, my_key: PublicKey) -> Result<BearerToken, Unauthorized> {
    let bin: Binary = token
        .0
        .parse()
        .map_err(|_| Unauthorized::InvalidBearerToken("Cannot parse token bytes.".into()))?;
    let msg = bin
        .as_ref()
        .try_into()
        .map_err(|_| Unauthorized::InvalidBearerToken("Not a signed token.".into()))?;
    store
        .read()
        .verify(&msg, vec![my_key])
        .map_err(|_| Unauthorized::InvalidSignature)?;
    let bearer_token = serde_cbor::from_slice::<BearerToken>(msg.message())
        .map_err(|_| Unauthorized::InvalidBearerToken("Cannot parse CBOR.".into()))?;
    Ok(bearer_token)
}

pub fn query_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::query()
        .map(|p: Params| p.token)
        .or_else(|_| async { Err(reject::custom(Unauthorized::MissingToken(TokenSource::QueryParam))) })
}

pub fn header_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::filters::header::optional("Authorization").and_then(|auth_header: Option<String>| async move {
        if let Some(auth_header) = auth_header {
            let mut words = auth_header.split_whitespace();
            let _ = match words.next() {
                Some("Bearer") => Ok(()),
                Some(auth_type) => Err(Unauthorized::UnsupportedAuthType(auth_type.to_string())),
                _ => Err(Unauthorized::UnsupportedAuthType("".to_string())),
            }?;
            let res: Result<Token, Rejection> = if let Some(token) = words.next() {
                Ok(Token(token.into()))
            } else {
                Err(Unauthorized::InvalidBearerToken("Missing token bytes.".into()).into())
            };
            res
        } else {
            Err(Unauthorized::MissingToken(TokenSource::Header).into())
        }
    })
}

pub fn authenticate(
    token: impl Filter<Extract = (Token,), Error = Rejection> + Clone,
    store: KeyStoreRef,
    my_key: PublicKey,
) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    token.and_then(move |t: Token| {
        let store = store.clone();
        async move {
            verify(t, store, my_key)
                .map(|bearer_token| bearer_token.app_id)
                .map_err(warp::reject::custom)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{app_id, Timestamp};
    use crypto::KeyStore;
    use parking_lot::RwLock;
    use serde_json::json;
    use std::sync::Arc;

    fn setup() -> (KeyStoreRef, PublicKey, Binary) {
        let mut store = KeyStore::default();
        let key_id = store.generate_key_pair().unwrap();
        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("test-app"),
            cycles: 0,
            version: "1.0.0".into(),
            validity: 300,
        };
        let bytes = serde_cbor::to_vec(&token).unwrap();
        let msg = store.sign(bytes, vec![key_id]).unwrap();
        let bearer = Binary::from(msg.as_ref());
        (Arc::new(RwLock::new(store)), key_id, bearer)
    }

    #[tokio::test]
    async fn should_work_with_header() {
        let (store, key_id, bearer) = setup();
        let route = authenticate(header_token(), store, key_id)
            .and(warp::path("p"))
            .map(|a: AppId| warp::reply::json(&json!(a.as_str())));
        let req = warp::test::request()
            .method("GET")
            .path("/p")
            .header("Authorization", format!("Bearer {}", bearer));
        let resp = req.reply(&route).await;
        assert!(resp.status().is_success());
        assert_eq!(resp.body(), "\"test-app\"");
    }

    #[derive(serde::Deserialize)]
    struct Query {
        token: Token,
    }

    #[tokio::test]
    async fn should_work_with_query_param() {
        let (store, key_id, bearer) = setup();
        let route = authenticate(warp::query().map(|x: Query| x.token), store, key_id)
            .and(warp::path("p"))
            .map(|a: AppId| warp::reply::json(&json!(a.as_str())));
        let req = warp::test::request()
            .method("GET")
            .path(format!("/p?token={}", bearer).replace("+", "%2B").as_str());
        let resp = req.reply(&route).await;
        assert!(resp.status().is_success());
        assert_eq!(resp.body(), "\"test-app\"");
    }

    #[tokio::test]
    async fn should_not_work() {
        let (store, key_id, bearer) = setup();
        // the token always starts with a few 'A's since the first bytes are zero (size field)
        // so we can invalidate the signature like so:
        let bearer = bearer.to_string().replace('A', "B");
        let route = authenticate(warp::header("Authorization").map(Token), store, key_id)
            .and(warp::path("p"))
            .map(|a: AppId| warp::reply::json(&json!(a.as_str())));
        let req = warp::test::request()
            .method("GET")
            .path("/p")
            .header("Authorization", bearer);
        let resp = req.reply(&route).await;
        assert!(resp.status().is_server_error()); // 500 unhandled rejection
    }
}
