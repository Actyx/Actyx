---
title: ax settings scopes
---

## Get setting scopes from a node

```
USAGE:
    ax settings scopes [FLAGS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here is a simple example of using the `ax settings scopes` command:

```
# Get the settings scopes from a node:
$ ax settings scopes --local 10.2.3.23
```
