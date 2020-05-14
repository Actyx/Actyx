---
title: ax settings scopes
---

## Get setting scopes from a node

```bash
$ ax settings scopes --help
USAGE: ax settings scopes [FLAGS] <NODE>

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <NODE>           Node ID or, if using `--local`, the IP address, of the node
                     to perform the operation on. You may also pass in a file with
                     a value on the first line using the syntax `@file.txt` or have
                     the command read one value per line from stdin using `@-`.
```

Here is a simple example of using the `ax settings scopes` command:

```bash
# Get the settings scopes from a node:
$ ax settings scopes --local 10.2.3.23
```
