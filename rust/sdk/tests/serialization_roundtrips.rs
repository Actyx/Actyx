use actyx_sdk::service::*;
use serde_json::*;

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug>(json: Value) -> anyhow::Result<()> {
    let value: T = from_value(json.clone())?;
    let serialized = to_value(value)?;
    anyhow::ensure!(json == serialized, "\nleft:  {:?}\nright: {:?}", json, serialized);
    Ok(())
}

#[test]
fn roundtrips() -> anyhow::Result<()> {
    roundtrip::<OffsetsResponse>(json!({
      "present": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 56
      },
      "toReplicate": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 1
      }
    }))?;

    roundtrip::<PublishRequest>(json!({
      "data": [
        {
          "tags": ["tag-01", "tag-02"],
          "payload": {
            "foo": { "a": 1, "b": 2 }
          }
        },
        {
          "tags": ["tag-02", "tag-03"],
          "payload": {
            "value": 42
          }
        }
      ]
    }))?;

    roundtrip::<PublishResponse>(json!({
      "data": [
        {
          "lamport": 84,
          "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
          "offset": 20,
          "timestamp": 1622110001582587u64
        },
        {
          "lamport": 85,
          "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
          "offset": 21,
          "timestamp": 1622110001582587u64
        }
      ]
    }))?;

    roundtrip::<QueryRequest>(json!({
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34
      },
      "upperBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 57
      },
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
      "order": "desc"
    }))?;

    roundtrip::<QueryResponse>(json!({
      "type": "event",
      "lamport": 28,
      "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
      "offset": 4,
      "appId": "com.actyx.test",
      "timestamp": 1622108806233884u64,
      "tags": ["tag-01", "tag-02"],
      "payload": {
        "value": 2
      }
    }))?;

    roundtrip::<SubscribeRequest>(json!({
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34,
      },
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
    }))?;

    roundtrip::<SubscribeResponse>(json!({
      "type": "event",
      "lamport": 28,
      "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
      "offset": 4,
      "appId": "com.actyx.test",
      "timestamp": 1622108806233884u64,
      "tags": ["tag-01", "tag-02"],
      "payload": {
        "value": 2
      }
    }))?;

    roundtrip::<SubscribeMonotonicRequest>(json!({
      "session": "my_session_id",
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34
      }
    }))?;

    roundtrip::<SubscribeMonotonicResponse>(json!({
      "type": "timeTravel",
      "newStart": {
        "lamport": 323,
        "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
        "offset": 34
      }
    }))?;

    roundtrip::<SubscribeMonotonicResponse>(json!({
      "type": "event",
      "lamport": 323,
      "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
      "offset": 34,
      "appId": "com.actyx.test",
      "timestamp": 1599224884528020u64,
      "tags": ["com.actyx.examples.temperature", "sensor:temp-sensor1"],
      "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
      },
      "caughtUp": true
    }))?;

    roundtrip::<NodeInfoResponse>(json!({
      "connectedNodes": 12,
      "uptime": {
        "secs": 1234,
        "nanos": 42,
      },
      "version": "Hello World Version",
      "swarmState": {
        "peersStatus": {
          "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s": "PartiallyWorking"
        }
      }
    }))
    .unwrap();

    Ok(())
}
