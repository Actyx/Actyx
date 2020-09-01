---
title: ax apps start
---

### Start an app on a node

```
USAGE:
    ax apps start [FLAGS] <APP> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <APP>     App ID of the app to start on the given node
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here are a couple of example of using the `ax apps start` command:

```
# Start a single app on a single node
ax apps start --local com.example.app 10.2.3.23
com.example.app successfully started on 10.2.3.23

# Start multiple apps using stdin
echo "com.example.myapp1
com.example.myapp2" | ax apps start --local @- 10.2.3.23

# Start a single app that is already running
ax apps start --local com.example.app 10.2.3.23
com.example.app is already running on 10.2.3.23
```
