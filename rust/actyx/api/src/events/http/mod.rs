mod filters;
mod handlers;
mod ndjson;

use warp::Filter;

use crate::events::service::EventService;
use crate::util::{filters::header_token, NodeInfo};

pub(crate) fn routes(
    auth_args: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let auth = crate::util::filters::authenticate(auth_args, header_token());
    filters::offsets(event_service.clone(), auth.clone())
        .or(filters::publish(event_service.clone(), auth.clone()))
        .or(filters::query(event_service.clone(), auth.clone()))
        .or(filters::subscribe(event_service.clone(), auth.clone()))
        .or(filters::subscribe_monotonic(event_service, auth))
}
