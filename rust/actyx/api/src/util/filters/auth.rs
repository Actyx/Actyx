use actyxos_sdk::{types::Binary, AppId};
use crypto::SignedMessage;
use std::convert::TryInto;
use warp::{reject, Filter, Rejection};

use crate::util::{AuthArgs, Token};
use crate::{rejections::ApiError, BearerToken};

pub(crate) fn verify(auth_args: AuthArgs, token: Token) -> Result<BearerToken, ApiError> {
    let token = token.to_string();
    let bin: Binary = token.parse().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Cannot parse token bytes.".to_owned(),
    })?;
    let signed_msg: SignedMessage = bin.as_ref().try_into().map_err(|_| ApiError::TokenInvalid {
        token: token.clone(),
        msg: "Not a signed token.".to_owned(),
    })?;
    auth_args
        .key_store
        .read()
        .verify(&signed_msg, vec![auth_args.node_key])
        .map_err(|_| ApiError::TokenUnauthorized)?;
    let bearer_token =
        serde_cbor::from_slice::<BearerToken>(signed_msg.message()).map_err(|_| ApiError::TokenInvalid {
            token: token.clone(),
            msg: "Cannot parse CBOR.".to_owned(),
        })?;
    match bearer_token.cycles != auth_args.cycles || bearer_token.is_expired() {
        true => Err(ApiError::TokenExpired),
        false => Ok(bearer_token),
    }
}

pub fn query_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::query::raw()
        .map(Token)
        .or_else(|_| async { Err(reject::custom(ApiError::MissingTokenParameter)) })
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
            Err(ApiError::MissingAuthorizationHeader.into())
        }
    })
}

pub(crate) fn authenticate(
    auth_args: AuthArgs,
    token: impl Filter<Extract = (Token,), Error = Rejection> + Clone,
) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    token.and_then(move |t: Token| {
        let auth_args = auth_args.clone();
        async move {
            verify(auth_args, t)
                .map(|bearer_token| bearer_token.app_id)
                // TODO: add necessary checks for the flow from the PRD
                .map_err(warp::reject::custom)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppMode;
    use actyxos_sdk::{app_id, Timestamp};
    use crypto::KeyStore;
    use parking_lot::RwLock;
    use std::sync::Arc;

    fn setup(validity: Option<u32>) -> (AuthArgs, Binary) {
        let mut store = KeyStore::default();
        let key_id = store.generate_key_pair().unwrap();
        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("test-app"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: validity.unwrap_or(300),
            app_mode: AppMode::Signed,
        };
        let bytes = serde_cbor::to_vec(&token).unwrap();
        let msg = store.sign(bytes, vec![key_id]).unwrap();
        let bearer = Binary::from(msg.as_ref());
        let auth_args = AuthArgs {
            cycles: 0.into(),
            key_store: Arc::new(RwLock::new(store)),
            node_key: key_id,
            token_validity: 300,
        };

        (auth_args, bearer)
    }

    #[tokio::test]
    async fn should_work_with_header() {
        let (auth_args, bearer) = setup(None);
        let filter = authenticate(auth_args, header_token());
        let req = warp::test::request().header("Authorization", format!("Bearer {}", bearer));
        assert_eq!(req.filter(&filter).await.unwrap(), app_id!("test-app"));
    }

    #[tokio::test]
    async fn should_fail_when_token_has_expired() {
        let (auth_args, bearer) = setup(Some(0));
        let filter = authenticate(auth_args, header_token());
        let req = warp::test::request()
            .header("Authorization", format!("Bearer {}", bearer))
            .filter(&filter)
            .await
            .unwrap_err();
        assert!(matches!(req.find::<ApiError>().unwrap(), ApiError::TokenExpired));
    }

    #[tokio::test]
    async fn should_fail_when_node_is_cycled() {
        let (auth_args, bearer) = setup(None);
        let auth_args = AuthArgs {
            // Simulate node restart
            cycles: 1.into(),
            ..auth_args
        };
        let filter = authenticate(auth_args, header_token());
        let req = warp::test::request()
            .header("Authorization", format!("Bearer {}", bearer))
            .filter(&filter)
            .await
            .unwrap_err();
        assert!(matches!(req.find::<ApiError>().unwrap(), ApiError::TokenExpired));
    }

    #[tokio::test]
    async fn should_work_with_query_param() {
        let (auth_args, bearer) = setup(None);
        let filter = authenticate(auth_args, query_token());
        let req = warp::test::request().path(&format!("/p?{}", bearer));
        assert_eq!(req.filter(&filter).await.unwrap(), app_id!("test-app"));
    }

    #[tokio::test]
    async fn should_not_work() {
        let (auth_args, bearer) = setup(None);
        // the token always starts with a few 'A's since the first bytes are zero (size field)
        // so we can invalidate the signature like so:
        let bearer = bearer.to_string().replace('A', "B");
        let filter = authenticate(auth_args, warp::header("Authorization").map(Token));
        let req = warp::test::request().header("Authorization", bearer);
        assert!(req.filter(&filter).await.is_err());
    }
}
