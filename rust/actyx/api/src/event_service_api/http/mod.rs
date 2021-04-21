mod filters;
mod handlers;
mod ndjson;

use actyxos_sdk::{service::EventService};
use warp::Filter;

use crate::util::{AuthArgs, filters::header_token};

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    auth_args: AuthArgs,
    event_service: S,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let auth = crate::util::filters::authenticate(auth_args, header_token());

    filters::node_id(event_service.clone(), auth.clone())
        .or(filters::offsets(event_service.clone(), auth.clone()))
        .or(filters::publish(event_service.clone(), auth.clone()))
        .or(filters::query(event_service.clone(), auth.clone()))
        .or(filters::subscribe(event_service.clone(), auth.clone()))
        .or(filters::subscribe_monotonic(event_service, auth))
}
