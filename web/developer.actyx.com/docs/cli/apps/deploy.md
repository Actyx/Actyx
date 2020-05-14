---
title: ax apps deploy
---

### Deploy apps to nodes

```bash
$ ax apps deploy --help
USAGE: ax apps deploy [FLAGS] <PATH> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    --force          Force update even if version number hasn't changed

ARGS:
    <PATH>           Path to the app directory or tarballs to process. Use
                     `.` for using the current working directory. You may also
                     pass in a file with a value on each line using the syntax
                     `@file.txt` or have the command read one value per line
                     from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

See the following example usages of the `ax apps deploy` command:

```bash
# Deploy the app in the current directory
$ ax apps deploy --local . 10.2.3.23

# Deploy a packaged tarball to a node
$ ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23

# Deploy an app to multiple nodes
$ ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23 10.2.3.24

# Deploy multiple apps to a node
$ echo "myApp1-1.0.0.tar.gz
myApp2-1.0.0.tar.gz" | ax apps deploy --local @- 10.2.3.23

# Deploy multiple apps to multiple nodes
$ cat apps.txt
myApp1-1.0.0.tar.gz
myApp2-1.0.0.tar.gz
$ cat nodes.txt
10.2.3.23
10.2.3.24
$ ax apps deploy --local @apps.txt @nodes.txt
```
