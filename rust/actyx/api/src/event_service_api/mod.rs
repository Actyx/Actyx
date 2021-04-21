mod http;
pub mod service;
mod ws;

use actyxos_sdk::service::EventService;
use warp::*;

use crate::util::AuthArgs;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    auth_args: AuthArgs,
    event_service: S,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(auth_args.clone(), event_service.clone()).or(ws::routes(auth_args, event_service))
}
