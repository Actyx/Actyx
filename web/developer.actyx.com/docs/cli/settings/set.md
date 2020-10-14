---
title: ax settings set
---

## Configure settings of a node

```text
USAGE:
    ax settings set [FLAGS] <SCOPE> <VALUE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <SCOPE>    Scope for which you want to set the given settings
    <VALUE>    The value you want to set for the given scope as a YAML or JSON
               string. You may also pass in a file using the syntax `@file.yml`
               or have the command read from stdin using `@-`
    <NODE>     Node ID or, if using `--local`, the IP address of the node to
               perform the operation on
```

:::cautionApp must be stopped in order to set settings for it
Nodes will only accept new settings if the relevant app is not running. For example: setting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).
:::

:::cautionAll app must be stopped in order to set node settings
Nodes will only accept new settings for the `com.actyx.os` scope if all apps on the node(s) are not running.
:::

Please see the following usage examples for the `ax settings set` command:

```text
# Set settings for settings scope com.actyx.os at node 10.2.3.23 from file
# node1.settings.yml
ax settings set --local com.actyx.os @node1.settings.yml 10.2.3.23

# Just set the specific setting displayName
ax settings set --local com.actyx.os/general/displayName "Node 1" 10.2.3.23

# Read in settings from stdin
echo "
general:
  swarmKey: 4245c0e542a4f89985a92de178d2169dc7f3596a382828aa8381bc13370e9880
  displayName: My Node
  bootstrapNodes:
    - /ipfs/10.24.24.2
    - /ipfs/10.24.24.3" | ax settings set --local com.actyx.os @- 10.2.3.23
```
