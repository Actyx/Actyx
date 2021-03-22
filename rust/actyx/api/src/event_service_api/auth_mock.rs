use actyxos_sdk::{app_id, AppId};
use crypto::{KeyStoreRef, PublicKey};
use futures::future;
use warp::*;

use crate::{rejections::ApiError, util::Token};

pub fn authenticate(
    token: impl Filter<Extract = (Token,), Error = Rejection> + Clone,
    _key_store: KeyStoreRef,
    _key: PublicKey,
) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    token.and_then(move |t: Token| match t.0.as_str() {
        "disallow" => future::err(reject::custom(ApiError::TokenUnauthorized)),
        "invalid" => future::err(reject::custom(ApiError::TokenInvalid {
            token: t.0,
            msg: "Cannot parse token bytes.".to_owned(),
        })),
        _ => future::ok(app_id!("placeholder.app")),
    })
}
