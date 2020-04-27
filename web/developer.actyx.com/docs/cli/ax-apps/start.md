---
title: ax apps start
---

### Start apps on nodes

```bash
$ ax apps start --help
USAGE: ax apps start [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <APP>            IDs of the app(s) to start on the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

Here are a couple of example of using the `ax apps start` command:

```bash
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
