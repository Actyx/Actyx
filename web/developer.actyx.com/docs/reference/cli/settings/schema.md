---
title: ax settings schema
---

```text title="Get setting schemas from a node"
USAGE:
    ax settings schema [FLAGS] [OPTIONS] <SCOPE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <SCOPE>    Scope from which you want to get the schema
    <NODE>     Node ID or the IP address of the node to
               perform the operation on
```

Here are some examples of using the `ax settings schema` command:

```text title="Example Usage"
# Get the settings schema for the node with settings scope com.actyx at 10.2.3.23
ax settings schema com.actyx 10.2.3.23
```
