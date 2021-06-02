---
title: ax nodes inspect
---

```text title="Show node details and connections"
USAGE:
    ax nodes inspect [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read [default:
                                 /Users/maximilianhaushofer/Library/Application Support/actyx/keys/users/id]

ARGS:
    <NODE>    Node ID or, if using `--local`, the IP address of the node to perform the operation on
```

The output will show you:
- `PeerId`: Peer ID of your node
- `ListenAddrs`: Addresses that the Swarm API bound to
- `AnnounceAddrs`: Addresses that your node is announcing to other nodes for connecting to it
- `Connections`: List of active connections to peers, identified by peer ID and address
- `knownPeers`: List of all peers, identified by peer ID and address, that your node knows

```text title="Example Usage"
ax nodes inspect --local 192.168.1.219
PeerId: 12D3KooWSgvc3hzrsuExYazNDB1BU3gevUPTzaumnwHWv5yFBNzH
ListenAddrs:
    /ip4/192.168.1.219/tcp/4001
    /ip4/127.0.0.1/tcp/4001
    /ip6/::1/tcp/4001
AnnounceAddrs:
    /ip4/192.168.1.219/tcp/4001/p2p/12D3KooWSgvc3hzrsuExYazNDB1BU3gevUPTzaumnwHWv5yFBNzH
Connections:
+------------------------------------------------------+--------------------------------------------------------------------------------------+
| PEERID                                               | ADDRESS                                                                              |
+------------------------------------------------------+--------------------------------------------------------------------------------------+
| 12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt | /ip4/192.168.1.165/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt |
+------------------------------------------------------+--------------------------------------------------------------------------------------+

Addresses:
+------------------------------------------------------+--------------------------------------------------------------------------------------+
| PEERID                                               | ADDRESS                                                                              |
+------------------------------------------------------+--------------------------------------------------------------------------------------+
| 12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt | /ip6/::1/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt           |
|                                                      | /ip4/192.168.1.165/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt |
|                                                      | /ip4/127.0.0.1/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt     |
+------------------------------------------------------+--------------------------------------------------------------------------------------+

# Get the output as a JSON object
ax -j nodes inspect --local 192.168.1.219 | jq .
{
  "code": "OK",
  "result": {
    "peerId": "12D3KooWSgvc3hzrsuExYazNDB1BU3gevUPTzaumnwHWv5yFBNzH",
    "listenAddrs": [
      "/ip4/192.168.1.219/tcp/4001",
      "/ip4/127.0.0.1/tcp/4001",
      "/ip6/::1/tcp/4001"
    ],
    "announceAddrs": [
      "/ip4/192.168.1.219/tcp/4001/p2p/12D3KooWSgvc3hzrsuExYazNDB1BU3gevUPTzaumnwHWv5yFBNzH"
    ],
    "connections": [
      {
        "peerId": "12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt",
        "addr": "/ip4/192.168.1.165/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt"
      }
    ],
    "peers": [
      {
        "peerId": "12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt",
        "addrs": [
          "/ip6/::1/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt",
          "/ip4/192.168.1.165/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt",
          "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWSVZEwAqdcJEG2T3wR8CZZoneWFyPqpRsxSzVB3WLwtVt"
        ]
      }
    ]
  }
}
```

