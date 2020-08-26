---
title: ax settings get
---

## Get settings from a node

```
USAGE:
    ax settings get [FLAGS] <SCOPE> <NODE>

FLAGS:
    -h, --help           Prints help information
    -l, --local          Process over local network
        --no-defaults    Only return settings explicitly set by the user and
                         skip default values
    -V, --version        Prints version information
    -v                   Verbosity level. Add more v for higher verbosity
                         (-v, -vv, -vvv, etc.)

ARGS:
    <SCOPE>    Scope from which you want to get the settings
    <NODE>     Node ID or, if using `--local`, the IP address of the node to
               perform the operation on
```

> Null or schema-conformant
>
> The return value of this command will always be either null or schema-conformant, unless the `--no-defaults` flag is used.

Here are a couple of examples of using the `ax settings get` command:

```
# Get the ActyxOS settings from a node:
$ ax settings get --local com.actyx.os 10.2.3.23

# Get the display name set for a node
$ ax settings get --local com.actyx.os/general/displayName 10.2.3.23

# Get the settings of a specific app
$ ax settings get --local com.example.app1 10.2.3.23

#Get a specific setting from an app
$ ax settings get --local com.example.app1/setting1 10.2.3.23
```
