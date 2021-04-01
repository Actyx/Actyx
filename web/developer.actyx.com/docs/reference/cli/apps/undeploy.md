---
title: ax apps undeploy
---

<!-- TODO NKI: replace with correct link -->

:::warning `ax apps` subcommand is deprecated
Managing apps via the Actyx CLI is deprecated and will no longer be supported in future ActyxOS versions.
For more information on this, please refer to [this guide](../../how-to-guides/configuring-and-packaging/actyx-swarms.mdx)
:::

```text title="Undeploy an app from an ActyxOS node"
USAGE:
    ax apps undeploy [FLAGS] <APP> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <APP>     App ID of the app to undeploy from the given node
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here is an example of using the `ax apps undeploy` command:

```text title="Example Usage"
# Undeploy app with ID com.example.app from node at 10.2.3.23
ax apps undeploy --local com.example.app 10.2.3.23
```
