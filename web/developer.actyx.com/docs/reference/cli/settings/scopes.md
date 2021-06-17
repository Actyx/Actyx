---
title: ax settings scopes
---

```text title="Get setting scopes from a node"
USAGE:
    ax settings scopes [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <NODE>    Node ID or the IP address of the node to
              perform the operation on
```

Here is an example of using the `ax settings scopes` command:

```text title="Example Usage"
# Get all the settings scopes from node at 10.2.3.23
ax settings scopes 10.2.3.23
```
