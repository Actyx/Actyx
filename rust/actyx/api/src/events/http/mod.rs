mod filters;
mod handlers;
mod ndjson;

use warp::Filter;

use crate::events::service::EventService;
use crate::util::NodeInfo;
use crate::{boxed_on_debug, or};

pub(crate) fn routes(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    or!(
        filters::offsets(node_info.clone(), event_service.clone()),
        filters::publish(node_info.clone(), event_service.clone()),
        filters::query(node_info.clone(), event_service.clone()),
        filters::subscribe(node_info.clone(), event_service.clone()),
        filters::subscribe_monotonic(node_info, event_service)
    )
}
