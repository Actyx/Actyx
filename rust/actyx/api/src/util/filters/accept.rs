use futures::future;
use warp::*;

use crate::rejections::ApiError;

pub fn accept(mime_types: &'static [&'static str]) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    header::optional("accept")
        .and_then(move |accept: Option<String>| match accept {
            Some(requested) if !mime_types.iter().any(|mt| *mt == requested.as_str()) => {
                future::err(reject::custom(ApiError::NotAcceptable {
                    requested,
                    supported: mime_types.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
                }))
            }
            _ => future::ok(()),
        })
        .untuple_one()
}
