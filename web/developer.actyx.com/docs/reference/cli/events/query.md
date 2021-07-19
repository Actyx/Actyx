---
title: ax events query
---

```text title="Query events currently know at a remote node"
USAGE:
    ax events query [FLAGS] [OPTIONS] <NODE> <query>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <NODE>     the IP address or <host>:<admin port> of the node to perform the operation on
    <query>    event API query
```

This command allows you to submit a query to a remote node, tunneled through the admin port.
For more details on the format of such queries please refer to [the API docs](../../events-api.mdx#query-event-streams).

```text title="Example usage"
$ ax events query localhost 'FROM "a"' | jq .
{
  "lamport": 8002,
  "stream": "eLaDx6r4VtctA3wVJnToJo.FIp.YinBoN5sL17CkvJU-0",
  "offset": 1,
  "timestamp": 1626157877703308,
  "tags": [
    "a",
    "b"
  ],
  "appId": "com.actyx.cli",
  "payload": 42
}
```

(The example above uses the [`jq`](https://stedolan.github.io/jq/) utility to nicely format the JSON output.)
