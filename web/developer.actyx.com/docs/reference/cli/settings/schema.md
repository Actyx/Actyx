---
title: ax settings schema
---

```text title="Get setting schemas from a node"
USAGE:
    ax settings schema [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <NODE>    the IP address or <host>:<admin port> of the node to perform the operation on
```

Here are some examples of using the `ax settings schema` command:

```text title="Example Usage"
# Get the settings schema for node at 10.2.3.23
ax settings schema / 10.2.3.23
```
