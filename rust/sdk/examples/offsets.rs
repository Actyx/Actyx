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
use actyx_sdk::{
    app_id,
    language::Query,
    service::{EventService, Order, QueryRequest, QueryResponse},
    AppManifest, HttpClient,
};
use futures::stream::StreamExt;
use url::Url;

async fn mk_http_client() -> anyhow::Result<HttpClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.actyx-offsets"),
        "Offsets Example".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse("http://localhost:4454").unwrap();
    HttpClient::new(url, app_manifest).await
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let service = mk_http_client().await?;

    // retrieve largest currently known event stream cursor
    let offsets = service.offsets().await?.present;
    println!("largest currently known event stream cursor {:#?}", offsets);

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let request = QueryRequest {
        lower_bound: None,
        upper_bound: offsets,
        query: "FROM 'sensor:temp-sensor1'".parse::<Query>()?,
        order: Order::Desc,
    };
    let mut events = service.query(request).await?;

    while let Some(QueryResponse::Event(event)) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
