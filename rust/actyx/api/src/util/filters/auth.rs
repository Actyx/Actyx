use actyxos_sdk::{types::Binary, AppId};
use crypto::{KeyStoreRef, PublicKey, SignedMessage};
use std::convert::TryInto;
use trees::BearerToken;
use warp::{reject, Filter, Rejection};

use crate::rejections::{ApiError, TokenSource};
use crate::util::{Params, Token};

#[allow(dead_code)]
fn verify(token: Token, store: KeyStoreRef, my_key: PublicKey) -> Result<BearerToken, ApiError> {
    let token = token.0;
    let bin: Binary = token.parse().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Cannot parse token bytes.".to_owned(),
    })?;
    let signed_msg: SignedMessage = bin.as_ref().try_into().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Not a signed token.".to_owned(),
    })?;
    store
        .read()
        .verify(&signed_msg, vec![my_key])
        .map_err(|_| ApiError::AppUnauthorized)?;
    let bearer_token =
        serde_cbor::from_slice::<BearerToken>(signed_msg.message()).map_err(|_| ApiError::TokenInvalid {
            token,
            msg: "Cannot parse CBOR.".to_owned(),
        })?;
    Ok(bearer_token)
}

pub fn query_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::query().map(|p: Params| p.token).or_else(|_| async {
        Err(reject::custom(ApiError::MissingAuthToken {
            source: TokenSource::QueryParam,
        }))
    })
}

pub fn header_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::filters::header::optional("Authorization").and_then(|auth_header: Option<String>| async move {
        if let Some(auth_header) = auth_header {
            let mut words = auth_header.split_whitespace();
            let _ = match words.next() {
                Some("Bearer") => Ok(()),
                Some(auth_type) => Err(ApiError::UnsupportedAuthType {
                    requested: auth_type.to_owned(),
                }),
                _ => Err(ApiError::UnsupportedAuthType {
                    requested: "".to_owned(),
                }),
            }?;
            let res: Result<Token, Rejection> = if let Some(token) = words.next() {
                Ok(Token(token.into()))
            } else {
                Err(ApiError::TokenInvalid {
                    token: "".to_owned(),
                    msg: "Missing token bytes.".to_owned(),
                }
                .into())
            };
            res
        } else {
            Err(ApiError::MissingAuthToken {
                source: TokenSource::Header,
            }
            .into())
        }
    })
}

#[allow(dead_code)]
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
        let filter = authenticate(header_token(), store, key_id);
        let req = warp::test::request().header("Authorization", format!("Bearer {}", bearer));
        assert_eq!(req.filter(&filter).await.unwrap(), app_id!("test-app"));
    }

    #[derive(serde::Deserialize)]
    struct Query {
        token: Token,
    }

    #[tokio::test]
    async fn should_work_with_query_param() {
        let (store, key_id, bearer) = setup();
        let filter = authenticate(warp::query().map(|x: Query| x.token), store, key_id);
        let req = warp::test::request().path(format!("/p?token={}", bearer).replace("+", "%2B").as_str());
        assert_eq!(req.filter(&filter).await.unwrap(), app_id!("test-app"));
    }

    #[tokio::test]
    async fn should_not_work() {
        let (store, key_id, bearer) = setup();
        // the token always starts with a few 'A's since the first bytes are zero (size field)
        // so we can invalidate the signature like so:
        let bearer = bearer.to_string().replace('A', "B");
        let filter = authenticate(warp::header("Authorization").map(Token), store, key_id);
        let req = warp::test::request().header("Authorization", bearer);
        assert!(req.filter(&filter).await.is_err());
    }
}
