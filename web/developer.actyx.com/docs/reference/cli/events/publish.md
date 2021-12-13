---
title: ax events publish
---

```text title="Publishing an event"
USAGE:
    ax events publish [FLAGS] [OPTIONS] <NODE> <payload>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <FILE>    File from which the identity (private key) for authentication is read
    -t, --tag <tag>...       tag (can be given multiple times)

ARGS:
    <NODE>       the IP address or <host>:<admin port> of the node to perform the operation on
    <payload>    event payload (JSON)
```

This command can be used to inject events into a running swarm.
The primary facility for doing this should always be the Actyx SDK, but sometimes mistakes need a quick fix before new app versions can be rolled out.
In such cases, please make very sure that your emitted event is correct, because you will not be able to remove it again.

```text
$ ax events publish localhost '["hello","world"]' -t mine -t other
published event ebMcEzA9FtHbFOsIPmHkyNZTyHZ./kglRlyAh3e6UXo-0/445 at 2021-12-09 12:44:05.807027 UTC

$ ax events query localhost 'FROM "mine"' | jq .
{
  "lamport": 892,
  "stream": "ebMcEzA9FtHbFOsIPmHkyNZTyHZ./kglRlyAh3e6UXo-0",
  "offset": 445,
  "timestamp": 1639053845807027,
  "tags": [
    "mine",
    "other"
  ],
  "appId": "com.actyx.cli",
  "payload": [
    "hello",
    "world"
  ]
}
```

As you can see, both tags are present.
The app ID always is `com.actyx.cli` for events emitted in this fashion.
