---
title: ax apps undeploy
---

### Undeploy apps from nodes

```bash
$ ax apps undeploy --help
USAGE: ax apps undeploy [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    -j or --json     Format output as JSON


ARGS:
    <APP>            IDs of the app(s) to undeploy from the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

Here are a couple of example of using the `ax apps undeploy` command:

```bash
# Undeploy a specific app from a node
$ ax apps undeploy --local com.example.myapp1 10.2.3.23

# Undeploy an app from multiple nodes
$ ax apps undeploy --local com.example.myapp1 10.2.3.23 10.2.3.24

# Undeploy multiple apps from a node
$ echo "com.example.myapp1
com.example.myapp2" | ax apps undeploy --local @- 10.2.3.23
```
