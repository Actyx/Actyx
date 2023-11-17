use actyx_sdk::service::{
    NodeInfoResponse, OffsetsResponse, PublishRequest, PublishResponse, QueryRequest, QueryResponse,
    SubscribeMonotonicRequest, SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
};
use serde_json::{from_value, json, to_value, Value};

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug>(json: Value) {
    let value: T = from_value(json.clone()).unwrap();
    let serialized = to_value(value).unwrap();
    assert_eq!(json, serialized)
}

#[test]
fn roundtrip_offsets_response() {
    roundtrip::<OffsetsResponse>(json!({
      "present": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 56
      },
      "toReplicate": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 1
      }
    }))
}

#[test]
fn roundtrip_publish_request() {
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
    }))
}

#[test]
fn roundtrip_publish_response() {
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
    }))
}

#[test]
fn roundtrip_query_request() {
    roundtrip::<QueryRequest>(json!({
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34
      },
      "upperBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 57
      },
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
      "order": "desc"
    }))
}

#[test]
fn roundtrip_query_response() {
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
    }))
}

#[test]
fn roundtrip_subscribe_request() {
    roundtrip::<SubscribeRequest>(json!({
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34,
      },
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
    }))
}

#[test]
fn roundtrip_subscribe_response() {
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
    }))
}

#[test]
fn roundtrip_subscribe_monotonic_request() {
    roundtrip::<SubscribeMonotonicRequest>(json!({
      "session": "my_session_id",
      "query": "FROM ('tag-01' & ('tag-02' | 'tag-03')) END",
      "lowerBound": {
        "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2": 34
      }
    }))
}

#[test]
fn roundtrip_subscribe_monotonic_timetravel() {
    roundtrip::<SubscribeMonotonicResponse>(json!({
      "type": "timeTravel",
      "newStart": {
        "lamport": 323,
        "stream": "1g1UOqdpvBB1KHsGWGZiK3Vi8MYGDZZ1oylpOajUk.s-2",
        "offset": 34
      }
    }))
}

#[test]
fn roundtrip_subscribe_monotonic_event() {
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
    }))
}

#[test]
fn roundtrip_node_info_response() {
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
}
