---
title: ax settings schema
---

```text title="Get setting schemas from a node"
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

Here are some examples of using the `ax settings schema` command:

```text title="Example Usage"
# Get the settings schema for the node with settings scope com.actyx at 10.2.3.23
ax settings schema --local com.actyx 10.2.3.23
```
