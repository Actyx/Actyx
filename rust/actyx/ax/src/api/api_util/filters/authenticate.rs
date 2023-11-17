use actyx_sdk::AppId;
use futures::FutureExt;
use tracing::{debug, info};
use warp::{reject, Filter, Rejection};

use crate::api::{
    api_util::{NodeInfo, Token},
    auth::verify_token,
    rejections::ApiError,
};

/// Tries to extract the value given to the `access_token` query parameter.
pub fn query_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::query::raw().and_then(|query_string: String| async move {
        let mut split = query_string.split('&');
        split
            .find_map(|x| {
                if x.starts_with("access_token=") {
                    Some(Token(x.trim_start_matches("access_token=").into()))
                } else {
                    None
                }
            })
            .ok_or_else(|| reject::custom(ApiError::MissingTokenParameter))
    })
}

/// Interpretes the whole query string as the access token. This method must only be used for the
/// WS connection!
pub fn query_token_ws() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::query::raw()
        .map(Token)
        .or_else(|_| async move { Err(reject::custom(ApiError::MissingTokenParameter)) })
}

pub fn header_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    warp::filters::header::optional("Authorization").and_then(|auth_header: Option<String>| async move {
        if let Some(auth_header) = auth_header {
            let mut words = auth_header.split_whitespace();
            match words.next() {
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

pub fn header_or_query_token() -> impl Filter<Extract = (Token,), Error = Rejection> + Clone {
    query_token().or(header_token()).unify()
}

pub fn header_or_query_token_opt() -> impl Filter<Extract = (Option<Token>,), Error = Rejection> + Clone {
    header_or_query_token()
        .map(Some)
        .recover(|e: Rejection| async move {
            if let Some(ApiError::MissingAuthorizationHeader) = e.find() {
                Result::<_, Rejection>::Ok(None)
            } else {
                Err(e)
            }
        })
        .unify()
}

pub(crate) fn authenticate_optional(
    node_info: NodeInfo,
    token: impl Filter<Extract = (Option<Token>,), Error = Rejection> + Clone,
) -> impl Filter<Extract = (Option<AppId>,), Error = Rejection> + Clone {
    token.and_then(move |t: Option<Token>| {
        if let Some(t) = t {
            let auth_args = node_info.clone();
            async move {
                let res = verify_token(auth_args, t)
                    .map(|bearer_token| bearer_token.app_id)
                    .map(Some)
                    // TODO: add necessary checks for the flow from the PRD
                    .map_err(warp::reject::custom);
                if res.is_err() {
                    info!("Auth failed: {:?}", res);
                } else {
                    debug!("Auth succeeded: {:?}", res);
                }
                res
            }
            .left_future()
        } else {
            async move { Ok(None) }.right_future()
        }
    })
}

pub(crate) fn authenticate(
    node_info: NodeInfo,
    token: impl Filter<Extract = (Token,), Error = Rejection> + Clone,
) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    token.and_then(move |t: Token| {
        let auth_args = node_info.clone();
        async move {
            let res = verify_token(auth_args, t)
                .map(|bearer_token| bearer_token.app_id)
                // TODO: add necessary checks for the flow from the PRD
                .map_err(warp::reject::custom);
            if res.is_err() {
                info!("Auth failed: {:?}", res);
            } else {
                debug!("Auth succeeded: {:?}", res);
            }
            res
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{formats::Licensing, AppMode, BearerToken},
        crypto::{KeyStore, PrivateKey},
    };
    use actyx_sdk::{app_id, types::Binary, Timestamp};
    use chrono::Utc;
    use parking_lot::RwLock;
    use std::sync::Arc;

    fn setup(validity: Option<u32>) -> (NodeInfo, Binary) {
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
        let auth_args = NodeInfo {
            cycles: 0.into(),
            key_store: Arc::new(RwLock::new(store)),
            node_id: key_id.into(),
            token_validity: 300,
            ax_public_key: PrivateKey::generate().into(),
            licensing: Licensing::default(),
            started_at: Utc::now(),
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
        let auth_args = NodeInfo {
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
        let req = warp::test::request().path(&format!("/?access_token={}&what=ever", bearer));
        assert_eq!(req.filter(&filter).await.unwrap(), app_id!("test-app"));
    }

    #[tokio::test]
    async fn should_work_with_query_param_legacy() {
        let (auth_args, bearer) = setup(None);
        let filter = authenticate(auth_args, query_token_ws());
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
