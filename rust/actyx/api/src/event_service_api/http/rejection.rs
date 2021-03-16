use warp::*;

#[derive(Debug)]
pub struct NotAcceptable {
    pub(crate) requested: String,
    pub(crate) supported: String,
}

impl reject::Reject for NotAcceptable {}

pub fn handle_rejection(r: Rejection) -> Result<impl Reply, Rejection> {
    match r.find() {
        Some(NotAcceptable { requested, supported }) => Ok(reply::with_status(
            format!(
                "The requested resource is only capable of generating content of type '{}' but '{}' was requested.",
                supported, requested
            ),
            http::StatusCode::NOT_ACCEPTABLE,
        )),
        _ => Err(r),
    }
}
