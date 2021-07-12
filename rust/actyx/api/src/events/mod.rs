mod http;
pub mod service;
mod ws;

use warp::*;

use crate::util::NodeInfo;
use service::EventService;

pub(crate) fn routes(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(node_info.clone(), event_service.clone()).or(ws::routes(node_info, event_service))
}
