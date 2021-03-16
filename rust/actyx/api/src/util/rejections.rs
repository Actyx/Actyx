#[derive(Debug)]
pub struct NotAcceptable {
    pub(crate) requested: String,
    pub(crate) supported: String,
}
impl warp::reject::Reject for NotAcceptable {}
