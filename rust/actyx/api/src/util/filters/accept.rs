use futures::future;
use warp::*;

use crate::rejections::ApiError;

pub fn accept(mime_types: &'static [&'static str]) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    let mime_types_normalized: std::collections::BTreeSet<_> = mime_types.iter().map(|m| m.to_lowercase()).collect();
    header::optional("accept")
        .and_then(move |accept: Option<String>| match accept {
            Some(requested)
                // TODO full content negotiation + q-factor weighting (preferably in warp)
                if !requested
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .any(|mt| mime_types_normalized.contains(&mt)) =>
            {
                future::err(reject::custom(ApiError::NotAcceptable {
                    requested,
                    supported: mime_types.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
                }))
            }
            _ => future::ok(()),
        })
        .untuple_one()
}

const ACCEPT_JSON: &[&str] = &["*/*", "application/json"];
pub fn accept_json() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    accept(ACCEPT_JSON)
}

const ACCEPT_NDJSON: &[&str] = &["*/*", "application/x-ndjson"];
pub fn accept_ndjson() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    accept(ACCEPT_NDJSON)
}

#[cfg(test)]
mod test {
    use super::*;

    async fn check(supported: &'static [&'static str], requested: &str, pass: bool) {
        assert_eq!(
            pass,
            warp::test::request()
                .header("Accept", requested)
                .filter(&accept(supported))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_accept() {
        check(&["application/json"], "application/json", true).await;
        check(&["application/json"], "Application/JSON", true).await;
        check(&["application/json"], "application/json, text/plain, */*", true).await;
        check(&["application/json"], "text/plain, application/json, */*", true).await;

        check(&["application/json"], "text/xml", false).await;
        check(&["application/json"], "text/xml, text/plain", false).await;
    }
}
