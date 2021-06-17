---
title: ax settings get
---

```text title="Get settings from a node"
USAGE:
    ax settings get [FLAGS] [OPTIONS] <SCOPE> <NODE>

FLAGS:
    -h, --help           Prints help information
    -j, --json           Format output as JSON
        --no-defaults    Only return settings explicitly set by the user and skip default values
    -V, --version        Prints version information
    -v                   Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <SCOPE>    Scope from which you want to get the settings
    <NODE>     the IP address or <host>:<admin port> of the node to perform the operation on
```

:::info Null or schema conformant
The return value of this command will always be either null or schema conformant, unless the `--no-defaults` flag is used.
:::

Here are some examples of using the `ax settings get` command:

```text title="Example Usage"
# Get the settings for the node at 10.2.3.23
# Use `/` for the root scope.
ax settings get / 10.2.3.23

# Just get the displayName setting
ax settings get /admin/displayName 10.2.3.23
```
