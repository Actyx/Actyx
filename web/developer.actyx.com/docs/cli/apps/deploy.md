---
title: ax apps deploy
---

### Deploy apps to nodes

```
$ ax apps deploy --help
USAGE: ax apps deploy [FLAGS] <PATH> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    --force          Force update even if version number hasn't changed
    -j or --json     Format output as JSON


OPTIONS:
    --arch <arch>    If you do not add the path to a file but to a directory,
                     ActyxOS will automatically package and deploy the app
                     that is in that directory. In case your app can be packaged for
                     more than one architecture, please specify the architecture
                     you want to deploy to (`aarch64` or `x86_64`).

ARGS:
    <PATH>           Path to the app directory or tarballs to process. Use
                     `.` for using the current working directory. You may also
                     pass in a file with a value on each line using the syntax
                     `@file.txt` or have the command read one value per line
                     from stdin using `@-`.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

See the following example usages of the `ax apps deploy` command:

```
# Deploy the app in the current directory
$ ax apps deploy --local . 10.2.3.23

# Deploy a packaged tarball to a node
$ ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23

# Deploy multiple apps to a node
$ echo "myApp1-1.0.0.tar.gz
myApp2-1.0.0.tar.gz" | ax apps deploy --local @- 10.2.3.23
```
