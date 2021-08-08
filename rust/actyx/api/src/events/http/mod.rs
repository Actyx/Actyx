mod filters;
mod handlers;
mod ndjson;

use warp::Filter;

use crate::events::service::EventService;
use crate::util::NodeInfo;

pub(crate) fn routes(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    filters::offsets(node_info.clone(), event_service.clone())
        .or(filters::publish(node_info.clone(), event_service.clone()))
        .or(filters::query(node_info.clone(), event_service.clone()))
        .or(filters::subscribe(node_info.clone(), event_service.clone()))
        .or(filters::subscribe_monotonic(node_info, event_service))
}
