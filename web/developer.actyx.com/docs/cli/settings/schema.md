---
title: ax settings schema
---

## Get setting schemas from a node

```
$ ax settings schema --help
USAGE: ax settings schema [FLAGS] <SCOPE> <NODE>

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    -j or --json     Format output as JSON

ARGS:
    <SCOPE>          Scope at which you want to get the settings.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

Here is a simple example of using the `ax settings schema` command:

```
# Get the ActyxOS nodes settings schema from a node
$ ax settings schema --local com.actyx.os 10.2.3.23

# Get the settings schema for a specific app from a node
$ ax settings schema --local com.example.app 10.2.3.23
```
