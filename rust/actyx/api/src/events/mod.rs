mod http;
pub mod service;
mod ws;

use actyx_sdk::service::EventService;
use warp::*;

use crate::util::NodeInfo;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    auth_args: NodeInfo,
    event_service: S,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(auth_args.clone(), event_service.clone()).or(ws::routes(auth_args, event_service))
}
