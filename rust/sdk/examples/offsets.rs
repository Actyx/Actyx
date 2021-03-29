/*
 * Copyright 2021 Actyx AG
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
#[allow(unused_imports)]
use actyxos_sdk::{
    service::{EventService, Order, QueryRequest, QueryResponse},
    HttpClient,
};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // client for locally running Actyx Event Service
    let service = HttpClient::default().await?;

    // retrieve largest currently known event stream cursor
    let offsets = service.offsets().await?;

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let mut events = service
        .query(QueryRequest {
            lower_bound: None,
            upper_bound: offsets,
            r#where: "MyFish".parse()?,
            order: Order::Desc,
        })
        .await?;

    // print out the payload of each event
    // (cf. Payload::extract for more options)
    while let Some(QueryResponse::Event(event)) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
