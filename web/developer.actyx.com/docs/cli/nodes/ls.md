---
title: ax nodes ls
---

<!-- markdownlint-disable-file MD040 -->

### Show node info and status

```
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

:::tip Output of `ax nodes ls`

If the node is reachable, the output of `ax nodes ls` will show you its status. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:

- Host unreachable
- ActyxOS unreachable (this means the host was reachable but the TCP connection reset)
:::

See the following examples of using the `ax nodes ls` command:

```
# get the status of all specified nodes in the local network
ax nodes ls --pretty --local 10.2.3.23 10.2.3.24 10.2.3.25
NODE ID    DISPLAY NAME  STATE   SETTINGS LICENSE  APPS DEPLOYED APPS RUNNING  STARTED                    VERSION
10.2.3.23  MY NODE       running    valid   valid             23           17  2020-03-18T06:17:00+01:00  1.0.0
10.2.3.24  ActyxOS unreachable
10.2.3.25  Host unreachable

# get the status of all nodes in the local network as a json object
ax --json nodes ls --local 10.2.3.23 10.2.3.24 10.2.3.25
{
    "code":"OK",
    "result": [
        {
            "connection":"reachable",
            "nodeId":"10.2.3.23",
            "displayName":"MY NODE",
            "state":"running",
            "settingsValid":true,
            "licensed":true,
            "appsDeployed":23,
            "appsRunning":17,
            "startedIso":"2020-05-19T07:52:26+00:00",
            "startedUnix":1589874746,
            "version":"1.0.0"
        },
        {
            "connection":"actyxosUnreachable",
            "host":"10.2.3.24"
        },
        {
            "connection":"hostosUnreachable",
            "host":"10.2.3.25"
        }
    ]
}
```

:::info `ax nodes ls` only returns the state of the node

Please keep in mind that **state**, **settings** and **license** in the  `ax nodes ls` command **only** refer to the node itself. If you want more detailed information about the state of the apps on a node, you need to use [`ax apps ls`](#apps-ls).
:::
