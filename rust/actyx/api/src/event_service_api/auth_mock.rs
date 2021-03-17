use actyxos_sdk::{app_id, AppId};
use crypto::{KeyStoreRef, PublicKey};
use futures::future;
use warp::*;

use crate::util::{rejections, Token};

pub fn authenticate(
    token: impl Filter<Extract = (Token,), Error = Rejection> + Clone,
    _key_store: KeyStoreRef,
    _key: PublicKey,
) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    token.and_then(move |t: Token| match t.0.as_str() {
        "disallow" => future::err(warp::reject::custom(rejections::Unauthorized::TokenUnauthorized)),
        "invalid" => future::err(warp::reject::custom(rejections::Unauthorized::InvalidBearerToken(t.0))),
        _ => future::ok(app_id!("placeholder.app")),
    })
}
