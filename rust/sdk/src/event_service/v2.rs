use super::{OffsetMap, Order, Payload};
use crate::tagged::{AppId, EventKey, TagSet};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub upper_bound: OffsetMap,
    pub subscription: String,
    pub order: Order,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub subscription: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiRequestBody {
    pub tags: TagSet,
    pub payload: Payload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiRequest {
    pub data: Vec<PublishApiRequestBody>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiResponseBody {
    pub app_id: AppId,
    pub key: EventKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiResponse {
    pub data: Vec<PublishApiResponseBody>,
}
