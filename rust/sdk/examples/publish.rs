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
use actyxos_sdk::{
    service::{EventService, PublishEvent, PublishRequest},
    tags, HttpClient, Payload,
};
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};

fn counter() -> impl Stream<Item = i32> {
    stream::iter(0..).then(|i| futures_timer::Delay::new(std::time::Duration::from_secs(1)).map(move |()| i))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = HttpClient::default().await?;
    let mut results = counter().flat_map(|i| {
        service
            .publish(PublishRequest {
                data: vec![
                    PublishEvent {
                        tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor1"),
                        payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                    },
                    PublishEvent {
                        tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor2"),
                        payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                    },
                ],
            })
            .into_stream()
    });

    while let Some(res) = results.try_next().await? {
        println!("{}", serde_json::to_value(res)?);
    }
    Ok(())
}
