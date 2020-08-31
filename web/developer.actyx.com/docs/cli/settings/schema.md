---
title: ax settings schema
---

<!-- markdownlint-disable-file MD040 -->

## Get setting schemas from a node

```
USAGE:
    ax settings schema [FLAGS] <SCOPE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <SCOPE>    Scope from which you want to get the schema
    <NODE>     Node ID or, if using `--local`, the IP address of the node to
               perform the operation on
```

Here is a simple example of using the `ax settings schema` command:

```
# Get the ActyxOS nodes settings schema from a node
ax settings schema --local com.actyx.os 10.2.3.23

# Get the settings schema for a specific app from a node
ax settings schema --local com.example.app 10.2.3.23
```
