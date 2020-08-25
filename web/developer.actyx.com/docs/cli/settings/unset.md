---
title: ax settings unset
---

## Unset settings on one or more nodes

```
$ ax settings unset --help
USAGE: ax settings unset [FLAGS] <SCOPE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    -j or --json     Format output as JSON

ARGS:
    <SCOPE>          Scope at which you want to set the given settings.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

:::cautionApp must be stopped in order to unset settings for it
Nodes will only unset an app's settings if the relevant app is not running. For example: unsetting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).
:::

:::cautionAll apps must be stopped in order to unset node settings
Nodes will only unset settings for the `com.actyx.os` scope if all apps on the node(s) are not running.
:::

Please see the following usage examples for the `ax settings unset` command:

```
# Unset ActyxOS settings from a node
$ ax settings unset --local com.actyx.os 10.2.3.23

# Unset a specific app's settings from a node
$ ax settings unset --local com.example.app 10.2.3.23

# Unset settings from multiple nodes
$ ax settings unset --local com.example.app 10.2.3.23 10.2.3.24

# Unset settings from multiple nodes defined in a file
$ ax settings unset --local com.example.app @nodes.txt
```
