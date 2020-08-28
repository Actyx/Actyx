---
title: ax apps undeploy
---

<!-- markdownlint-disable-file MD040 -->

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

Here are a couple of example of using the `ax apps undeploy` command:

```
# Undeploy a specific app from a node
ax apps undeploy --local com.example.myapp1 10.2.3.23

# Undeploy an app from multiple nodes
ax apps undeploy --local com.example.myapp1 10.2.3.23 10.2.3.24

# Undeploy multiple apps from a node
echo "com.example.myapp1
com.example.myapp2" | ax apps undeploy --local @- 10.2.3.23
```
