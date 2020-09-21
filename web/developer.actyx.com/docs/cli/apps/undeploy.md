---
title: ax apps undeploy
---

### Undeploy an app from a node

```
USAGE:
    ax apps undeploy [FLAGS] <APP> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <APP>     App ID of the app to undeploy from the given node
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here is an example of using the `ax apps undeploy` command:

```
# Undeploy app with ID com.example.app from node at 10.2.3.23
ax apps undeploy --local com.example.app 10.2.3.23
```
