---
title: ax settings unset
---

```text title="Remove settings from an ActyxOS node"
USAGE:
    ax settings unset [FLAGS] <SCOPE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <SCOPE>    Scope for which you want to unset the settings
    <NODE>     Node ID or, if using `--local`, the IP address of the node to
               perform the operation on
```

:::caution App must be stopped in order to unset settings for it
Nodes will only unset an app's settings if the relevant app is not running. For example: unsetting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).
:::

:::caution All apps must be stopped in order to unset node settings
Nodes will only unset settings for the `com.actyx.os` scope if all apps on the node(s) are not running.
:::

Please see the following usage examples for the `ax settings unset` command:

```text title="Example Usage"
# Unset the settings for the node with settings scope com.actyx.os at 10.2.3.23
ax settings unset --local com.actyx.os 10.2.3.23

# Unset the settings for the app with settings scope com.example.app from node
# at 10.2.3.23
ax settings unset --local com.example.app 10.2.3.23
```
