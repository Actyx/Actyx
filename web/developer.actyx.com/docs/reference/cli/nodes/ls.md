---
title: ax nodes ls
---

```text title="Show node info and status"
USAGE:
    ax nodes ls [FLAGS] <NODE>...

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <NODE>...    Node ID or, if using `--local`, the IP address of the node
                 to perform the operation on
```

If the node is reachable, the output of `ax nodes ls` will show you its status. If the node is unreachable, it is displayed as such in the output.
See the following examples of using the `ax nodes ls` command:

```text title="Example Usage"
# Get the status of a specified node in the local network
[17:50][~:]$ ax nodes ls -l localhost
+---------------------------------------------+--------------+-------------------------+---------------------------+---------+
| NODE ID                                     | DISPLAY NAME | HOST                    | STARTED                   | VERSION |
+---------------------------------------------+--------------+-------------------------+---------------------------+---------+
| GMRqsWDid4CrnGDbEVBul4biaaLYgBfz3Ou/5INDVFI | Default Node | /ip4/127.0.0.1/tcp/4458 | 2021-05-25T15:49:54+00:00 | 2.0.0   |
+---------------------------------------------+--------------+-------------------------+---------------------------+---------+

# Get the status of all specified nodes in the local network as a json object
[17:51][~:]$ ax --json nodes ls -l localhost 192.168.2.185 192.168.1.212 | jq
{
  "code": "OK",
  "result": [
    {
      "connection": "reachable",
      "host": "/ip4/127.0.0.1/tcp/4458",
      "nodeId": "GMRqsWDid4CrnGDbEVBul4biaaLYgBfz3Ou/5INDVFI",
      "displayName": "Default Node",
      "startedIso": "2021-05-25T15:49:54+00:00",
      "startedUnix": 1621957794,
      "version": {
        "profile": "release",
        "target": "macos-x86_64",
        "version": "2.0.0_dev",
        "gitHash": "b877bc357"
      }
    },
    {
      "connection": "unreachable",
      "host": "[192.168.2.185:4458]"
    },
    {
      "connection": "unreachable",
      "host": "[192.168.1.212:4458]"
    }
  ]
}
```
