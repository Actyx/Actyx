---
title: ax settings set
---

## Set settings on one or more nodes

```bash
$ ax settings set --help
USAGE: ax settings set [FLAGS] <SCOPE> <VALUE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <SCOPE>          Scope at which you want to set the given settings.
    <VALUE>          The value you want to set at the given scope as a YAML
                     or JSON string. You may also pass in a file using the
                     syntax `@file.yml` or have the command read from
                     stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

:::cautionApp must be stopped in order to set settings for it
Nodes will only accept new settings if the relevant app is not running. For example: setting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).
:::

:::cautionAll app must be stopped in order to set node settings
Nodes will only accept new settings for the `com.actyx.os` scope if all apps on the node(s) are not running.
Please see the following usage examples for the `ax settings set` command:
:::

```bash
# Setting for a node from a file
$ ax settings set --local com.actyx.os @Node1.settings.yml 10.2.3.23

# Setting a single specific ActyxOS node setting
$ ax settings set --local com.actyx.os/general/displayName "Node 1" 10.2.3.23

# Setting settings for multiple nodes at the same time
$ ax settings set --local com.actyx.os/general/swarmKey @swarm.key 10.2.3.23 10.2.3.24

# Setting ActyxOS settings for a node using stdin
$ echo "
general:
  swarmKey: 4245c0e542a4f89985a92de178d2169dc7f3596a382828aa8381bc13370e9880
  displayName: My Node
  bootstrapNodes:
    - /ipfs/10.24.24.2
    - /ipfs/10.24.24.3" | ax settings set --local com.actyx.os @- 10.2.3.23
```
