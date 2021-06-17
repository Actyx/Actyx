---
title: ax settings unset
---

```text title="Remove settings from a node"
USAGE:
    ax settings unset [FLAGS] [OPTIONS] <SCOPE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <SCOPE>    Scope for which you want to unset the settings
    <NODE>     IP address of the node to perform the operation on
```

Please see the following usage examples for the `ax settings unset` command:

```text title="Example Usage"
# Unset the settings for the node at 10.2.3.23
# Use `/` to unset entire node settings.
ax settings unset / 10.2.3.23
```
