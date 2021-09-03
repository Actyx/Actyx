mod query_executor;
mod radix_tree;
use std::{collections::BTreeSet, convert::TryFrom, str::FromStr};

use actyx_sdk::{
    service::{
        EventService, Order, PublishEvent, PublishRequest, QueryRequest, QueryResponse, SubscribeRequest,
        SubscribeResponse,
    },
    tag, tags, AppId, AppManifest, HttpClient, OffsetMap, Payload, Tag, TagSet,
};
use anyhow::Context;
use futures::{future, stream::StreamExt};
use radix_tree::RadixTree;
use serde::Serialize;
use structopt::StructOpt;
use url::Url;

use crate::query_executor::QueryExecutor;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Clone)]
struct TagMapper {
    mapping: RadixTree<String>,
}

impl FromStr for TagMapper {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self { mapping: text.parse()? })
    }
}

impl TagMapper {
    pub fn new(mapping: RadixTree<String>) -> Self {
        Self { mapping }
    }

    pub fn transform_tag_set(&self, ts: &TagSet) -> anyhow::Result<TagSet> {
        ts.iter()
            .map(|tag| {
                let text: String = self.mapping.substitute(tag.as_ref()).context("mapping not defined")?;
                Tag::try_from(text.as_ref()).context("mapped to empty tag")
            })
            .collect()
    }
}
#[derive(StructOpt, Debug)]
#[structopt(name = "converter")]
struct Opt {
    #[structopt(long, help("the app id to use for the converter"))]
    app_id: String,

    #[structopt(long, help("short name for where we are converting from"))]
    from: String,

    #[structopt(long, help("short name for where we are converting to"))]
    to: String,

    #[structopt(long, default_value = r#"{ "": "" }"#)]
    tag_mapping: String,

    #[structopt(long("query"), default_value = r#"FROM allEvents"#)]
    queries: Vec<String>,

    #[structopt(long)]
    doit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ConverterEvent {
    ConvertedUpTo(OffsetMap),
}

fn load_local(text: String) -> anyhow::Result<String> {
    if text.starts_with("@") {
        let filename = &text[1..];
        Ok(std::fs::read_to_string(filename)?)
    } else {
        Ok(text)
    }
}

fn format_tag_set(tags: &TagSet) -> String {
    let tags = tags.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    format!("{{{}}}", tags.join(" "))
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let Opt {
        app_id,
        from,
        to,
        tag_mapping,
        queries,
        doit,
    } = Opt::from_args();
    if !doit {
        println!("Dry run. To actually do it, pass --doit");
    }
    let app_id = AppId::try_from(app_id.as_ref()).context("Illegal app id")?;
    let tag_mapping = load_local(tag_mapping).context("unable to load tag mapping")?;
    let tag_mapping: RadixTree<String> = tag_mapping.parse().context("unable to parse tag mapping")?;
    let tag_mapping = tag_mapping.prefix("", "to/");
    let tag_mapper = TagMapper::new(tag_mapping);
    let executors = queries
        .into_iter()
        .map(|query| load_local(query))
        .map(|query| query.and_then(|q| q.parse::<QueryExecutor>()))
        .collect::<anyhow::Result<Vec<_>>>()?;
    // tag we add to all events we convert
    let converted_from_from = Tag::try_from(format!("converted-from:{}", from).as_ref())?;
    // tag we don't want to see
    let converted_from_to = Tag::try_from(format!("converted-from:{}", to).as_ref())?;
    // tag for the offsets
    let converted_up_to = tag!("converted-up-to");

    // add your app manifest, for brevity we will use one in trial mode
    let app_manifest = AppManifest::new(app_id, "Generic converter".into(), "0.1.0".into(), None);

    // Url of the locally running Actyx node
    let url = Url::parse("http://localhost:4454")?;
    // client for
    let service = HttpClient::new(url, app_manifest).await?;
    let extract_converted_up_to = |response: QueryResponse| async {
        if let QueryResponse::Event(e) = response {
            if e.tags.contains(&converted_up_to) {
                return Some(e.payload.extract::<OffsetMap>().unwrap());
            }
        };
        None
    };
    let lower_bound = service
        .query(QueryRequest {
            lower_bound: None,
            upper_bound: None,
            order: Order::StreamAsc,
            query: "FROM 'converted-up-to'".parse()?,
        })
        .await?
        .filter_map(extract_converted_up_to)
        .fold(OffsetMap::empty(), |a, b| future::ready(a.union(&b)))
        .await;
    tracing::info!("Startup: {:?}", lower_bound);

    let mut events = service
        .subscribe(SubscribeRequest {
            lower_bound: Some(lower_bound),
            query: "FROM allEvents".parse()?,
        })
        .await?
        .ready_chunks(64);

    // print out the payload of each event
    // (cf. Payload::extract for more options)
    while let Some(responses) = events.next().await {
        // these are just the offset for this chunk of events!
        let mut offsets = OffsetMap::empty();
        // union with possible offset maps
        for response in responses.iter() {
            if let SubscribeResponse::Offsets(update) = response {
                offsets.union_with(&update.offsets);
            }
        }
        let mut skipped = 0;
        let convert_relevant_events = |response: SubscribeResponse| {
            if let SubscribeResponse::Event(mut event) = response {
                // mark this event as converted
                offsets.update(event.stream, event.offset);
                // take only tags that start with "/from" and strip them
                let tags = event.tags.filter_prefix("from/").collect::<TagSet>();
                if !tags.is_empty() {
                    // panic if there are tags for which we have no mapping defined
                    println!("{:?} {:?} {:?}", tag_mapper, event.tags, tags);
                    let mut target_tags = tag_mapper.transform_tag_set(&tags).unwrap();
                    println!("{:?} {:?}", tags, target_tags);
                    let converted_from = event.tags.iter_prefix("converted-from:").collect::<BTreeSet<_>>();
                    if !converted_from.contains(&converted_from_to) {
                        for converted_from in converted_from {
                            target_tags.insert(converted_from);
                        }
                        target_tags.insert(converted_from_from.clone());
                        // transform the event
                        event.tags = tags;
                        for executor in &executors {
                            let result = executor.feed(&event, &target_tags);
                            tracing::info!("{} {}", executor, result.len());
                            // the first one that returns events wins!
                            if !result.is_empty() {
                                return result;
                            }
                        }
                    } else {
                        skipped += 1;
                        tracing::info!("Skipping event to prevent a loop '{:?}'", target_tags);
                    }
                }
            }
            Vec::new()
        };
        let mut events = responses
            .into_iter()
            .flat_map(convert_relevant_events)
            .collect::<Vec<_>>();
        if !events.is_empty() || skipped > 0 {
            tracing::info!(
                "Publishing {} converted events and new coverted_up_to with {} offset update",
                events.len(),
                offsets.streams().count()
            );
            events.push(PublishEvent {
                tags: tags!(converted_up_to.clone()),
                payload: Payload::compact(&offsets)?,
            });
            if doit {
                service.publish(PublishRequest { data: events }).await?;
            } else {
                tracing::info!("Dry run: would emit");
                for event in events {
                    tracing::info!(
                        "tags:{} data:{}",
                        format_tag_set(&event.tags),
                        event.payload.json_string()
                    );
                }
                tracing::info!("");
            }
        }
    }
    Ok(())
}
