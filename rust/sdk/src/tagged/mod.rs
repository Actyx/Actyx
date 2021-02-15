/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! Types and HTTP client for the Event Service v2 API with event tagging support
//!
//! The corresponding HTTP API in ActyxOS is not yet released, please refer to
//! the [`event_service`](../event_service/index.html) module in the meantime.
//!
//! The HTTP client is only available with the `client` feature flag.

#[cfg(any(test, feature = "arb"))]
mod arb;
#[cfg(feature = "client")]
mod client;
mod event;
mod scalars;
mod service;
mod snapshot;
mod tags;

#[cfg(feature = "client")]
pub use client::EventService;
pub use event::{Event, EventKey, Metadata};
pub use scalars::{AppId, NodeId, SessionId, StreamId, StreamNr};
pub use service::{
    QueryApiResponse, StartFrom, SubscribeApiResponse, SubscribeMonotonicRequest, SubscribeMonotonicResponse,
};
pub use snapshot::{Compression, SnapshotData};
pub use tags::{Tag, TagSet};
