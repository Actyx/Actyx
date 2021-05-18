---
title: ax settings unset
---

```text title="Remove settings from a node"
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

Please see the following usage examples for the `ax settings unset` command:

```text title="Example Usage"
# Unset the settings for the node with settings scope com.actyx at 10.2.3.23
ax settings unset --local com.actyx 10.2.3.23
```
