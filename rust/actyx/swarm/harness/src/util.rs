use crate::MachineExt;
use actyx_sdk::{
    app_id,
    service::{OffsetsResponse, PublishEvent, PublishRequest},
    AppManifest, Payload, TagSet,
};
use netsim_embed::Netsim;
use std::fmt::{Debug, Display};
use std::{collections::BTreeMap, str::FromStr};
use swarm_cli::Command;

pub fn app_manifest() -> AppManifest {
    AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    )
}

pub fn to_events(tags: Vec<TagSet>) -> Vec<(TagSet, Payload)> {
    tags.into_iter().map(|t| (t, Payload::empty())).collect()
}

pub fn to_publish(events: Vec<(TagSet, Payload)>) -> PublishRequest {
    PublishRequest {
        data: events
            .into_iter()
            .map(|(tags, payload)| PublishEvent { tags, payload })
            .collect(),
    }
}

pub fn format_offsets<E>(sim: &mut Netsim<Command, E>, offsets: OffsetsResponse) -> String
where
    E: FromStr + Send + 'static,
    E::Err: Debug + Display + Send + Sync + 'static,
{
    let mut to_replicate = offsets.to_replicate;
    let ids = sim
        .machines()
        .iter()
        .map(|m| (m.node_id(), m.id()))
        .collect::<BTreeMap<_, _>>();
    let mut lines = offsets
        .present
        .stream_iter()
        .map(|(s, o)| {
            let r: u64 = to_replicate.remove(&s).map(|x| x.into()).unwrap_or_default();
            let r = if r == 0 {
                "".to_string()
            } else {
                format!("(needs {})", r)
            };
            let n = ids
                .get(&s.node_id())
                .map(|m| format!("{}-{}", m, s.stream_nr()))
                .unwrap_or_else(|| s.to_string());
            format!("    {} -> {} {}", n, o, r)
        })
        .collect::<Vec<_>>();
    lines.sort();
    lines.join("\n")
}
