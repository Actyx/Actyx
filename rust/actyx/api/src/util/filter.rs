use futures::future;
use warp::*;

use crate::util::rejections::NotAcceptable;

pub fn accept(mime: &'static str) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    header::optional("accept")
        .and_then(move |accept: Option<String>| match accept {
            Some(requested) if requested.as_str() != mime => future::err(reject::custom(NotAcceptable {
                requested,
                supported: mime.to_owned(),
            })),
            _ => future::ok(()),
        })
        .untuple_one()
}
