---
title: ax events offsets
---

```text title="Query currently known event counts"
USAGE:
    ax events offsets [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <NODE>    the IP address or <host>:<admin port> of the node to perform the operation on
```

This command allows you to access a remote node’s admin port to find out how many events of which event stream it has already replicated.
The listed streams are named by the node ID of their origin with a stream number attached, for example like the following:

```text title="Example usage"
[17:50][~:]$ ax events offsets localhost
┌───────────────────────────────────────────────┬────────┬──────────────┐
│ STREAM ID                                     │ OFFSET │ TO REPLICATE │
├───────────────────────────────────────────────┼────────┼──────────────┤
│ 51tJcIrv6ijBeurYcn/H7/OWWYAdPXufYYB6KMqg3/Y-1 │ 5      │ +3           │
│ eLaDx6r4VtctA3wVJnToJo.FIp.YinBoN5sL17CkvJU-0 │ 1      │              │
│ eLaDx6r4VtctA3wVJnToJo.FIp.YinBoN5sL17CkvJU-1 │ 247    │              │
│ eLaDx6r4VtctA3wVJnToJo.FIp.YinBoN5sL17CkvJU-2 │ 8116   │              │
└───────────────────────────────────────────────┴────────┴──────────────┘
```

In this example we see three streams from one node (in this case the one we connected to) and one stream from another node.
That other node has previously emitted three more events that have not yet been successfully transferred to the node we queried.

This command is mainly used as a debugging tool to see which events have made it how far across the Actyx swarm.
