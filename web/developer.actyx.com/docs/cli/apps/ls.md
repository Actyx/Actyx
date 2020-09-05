---
title: ax apps ls
---

### List apps deployed on a node

```
USAGE:
    ax apps ls [FLAGS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

:::tip Output
If a node is reachable, the output of `ax apps ls` will list the status of all apps deployed on that node. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:

- Host unreachable
- ActyxOS unreachable (this means the host was reachable but the TCP connection reset)
:::

See the following examples of using the `ax apps ls` command:

```
# List the apps on a node in your local network
ax apps ls --local 10.2.3.23
+---------------+------------------------+---------+---------+---------+----------+---------+-------------------------------------+
| NODE ID       | APP ID                 | VERSION | ENABLED | STATE   | SETTINGS | LICENSE | STARTED                             |
+---------------+------------------------+---------+---------+---------+----------+---------+-------------------------------------+
| 10.2.3.23     | com.actyx.mwl          | 1.0.0   | ENABLED | RUNNING | VALID    | VALID   | 2020-09-01T15:24:03.816870152+00:00 |
+---------------+------------------------+---------+---------+---------+----------+---------+-------------------------------------+

# Get the status of apps on a node in the local network as a JSON object
ax --json apps ls --local 10.2.3.23
{
    "code":"OK",
    "result": [
        {
            "nodeId":"10.2.3.23",
            "appId":"com.actyx.mwl",
            "version":"1.0.0",
            "running":true,
            "startedIso":"2020-05-19T07:52:35.315693528+00:00",
            "startedUnix":1589874755,
            "licensed":true,
            "settingsValid":true,
            "enabled":true
        }
    ]
}
````

:::info`ax apps ls` only returns the state of the apps

Please keep in mind that **state**, **settings** and **license** in the  `ax apps ls` command **only** refer to the apps deployed on a node. If you want more detailed information about the node itself, you need to use [`ax nodes ls`](../nodes/ls).
:::
