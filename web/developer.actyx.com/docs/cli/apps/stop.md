---
title: ax apps stop
---

### Stop an app on a node

```
USAGE:
    ax apps stop [FLAGS] <APP> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <APP>     App ID of the app to stop on the given node
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here are a couple of example of using the `ax apps stop` command:

```
# Stop app with App ID com.example.app on node at 10.2.3.23
ax apps stop --local com.example.app 10.2.3.23
```
