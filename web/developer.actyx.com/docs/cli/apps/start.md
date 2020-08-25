---
title: ax apps start
---

### Start apps on nodes

```
$ ax apps start --help
USAGE: ax apps start [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    -j or --json     Format output as JSON


ARGS:
    <APP>            IDs of the app(s) to start on the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

Here are a couple of example of using the `ax apps start` command:

```
# Start a single app on a single node
$ ax apps start --local com.example.app 10.2.3.23
com.example.app successfully started on 10.2.3.23

# Start multiple apps using stdin
$ echo "com.example.myapp1
com.example.myapp2" | ax apps start --local @- 10.2.3.23

# Start a single app that is already running
$ ax apps start --local com.example.app 10.2.3.23
com.example.app is already running on 10.2.3.23
```
