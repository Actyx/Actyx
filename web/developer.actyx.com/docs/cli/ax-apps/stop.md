---
title: ax apps stop
---

### Stop apps on nodes

```bash
$ ax apps stop --help
USAGE: ax apps stop [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <APP>            IDs of the app(s) to stop on the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

Here are a couple of example of using the `ax apps stop` command:

```bash
# Stop a single app on a single node
$ ax apps stop --local com.example.app 10.2.3.23

# Stop multiple apps using stdin
$ echo "com.example.myapp1
com.example.myapp2" | ax apps stop --local @- 10.2.3.23
```
