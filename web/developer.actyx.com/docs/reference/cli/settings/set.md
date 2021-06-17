---
title: ax settings set
---

```text title="Configure settings of a node"
USAGE:
    ax settings set [FLAGS] [OPTIONS] <SCOPE> <VALUE> <NODE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <identity>    File from which the identity (private key) for authentication is read

ARGS:
    <SCOPE>    Scope for which you want to set the given settings
    <VALUE>    The value you want to set for the given scope as a YAML or JSON
               string. You may also pass in a file using the syntax `@file.yml`
               or have the command read from stdin using `@-`
    <NODE>     Node ID or the IP address of the node to perform the operation on
```

Please see the following usage examples for the `ax settings set` command:

```text title="Example Usage"
# Set settings for settings scope com.actyx at node 10.2.3.23 from file
# node1.settings.yml
ax settings set com.actyx @node1.settings.yml 10.2.3.23

# Just set the specific setting displayName
ax settings set com.actyx/admin/displayName "Node 1" 10.2.3.23

# Read in settings from stdin
echo "
admin:
  displayName: My Node
swarm:
  swarmKey: 4245c0e542a4f89985a92de178d2169dc7f3596a382828aa8381bc13370e9880
  initialPeers:
    - /ipfs/10.24.24.2
    - /ipfs/10.24.24.3" | ax settings set com.actyx @- 10.2.3.23
```
