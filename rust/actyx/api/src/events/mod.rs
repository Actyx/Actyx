mod http;
pub mod service;
mod ws;

use warp::*;

use crate::events::service::EventService;
use crate::util::NodeInfo;

pub(crate) fn routes(
    auth_args: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(auth_args.clone(), event_service.clone()).or(ws::routes(auth_args, event_service))
}
