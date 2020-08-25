---
title: ax apps ls
---

### Get the status of apps that are deployed to your node

```
$ ax apps ls --help
USAGE: ax apps ls [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -v, -vv, -vvv        Increase verbosity
    -h, --help           Prints help information
    --local              Process over local network
    -j or --json         Format output as JSON

ARGS:
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command read the value from stdin using `@-`.
```

:::tip Output
If a node is reachable, the output of `ax apps ls` will list the status of all apps deployed on that node. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:

- Host unreachable
- ActyxOS unreachable (this means the host was reachable but the TCP connection reset)
:::

See the following examples of using the `ax apps ls` command:

```
# List the apps on a node in your local network
$ ax apps ls --local 10.2.3.23

NODE ID    APP ID         STATE    SETTINGS LICENSE  MODE      STARTED                    VERSION
10.2.3.23  com.actyx.mwl  running     valid   valid  enabled   2020-03-18T06:17:00+01:00  1.0.0

# Get the status of apps on a node in the local network as a JSON object
$ ax --json apps ls --local 10.2.3.23
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

# Use an address in a file
$ ax apps ls --local @address.txt

# Pass the address from stdin
$ echo "10.2.3.23" | ax apps ls --local @-

````

:::info`ax apps ls` only returns the state of the apps

Please keep in mind that **state**, **settings** and **license** in the  `ax apps ls` command **only** refer to the apps deployed on a node. If you want more detailed information about the node itself, you need to use [`ax nodes ls`](../nodes/ls).
:::
