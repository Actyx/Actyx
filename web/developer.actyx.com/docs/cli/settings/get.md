---
title: ax settings get
---

## Get settings from a node

```bash
$ ax settings get --help
USAGE: ax settings get [FLAGS] <SCOPE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    --no-defaults    Do not return non-user-defined defaults from the schema
    -j or --json     Format output as JSON

ARGS:
    <SCOPE>          Scope at which you want to get the settings.
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the
                     command one value from stdin using `@-`.
```

> Null or schema-conformant
>
> The return value of this command will always be either null or schema-conformant, unless the `--no-defaults` flag is used.

Here are a couple of examples of using the `ax settings get` command:

```bash
# Get the ActyxOS settings from a node:
$ ax settings get --local com.actyx.os 10.2.3.23

# Get the display name set for a node
$ ax settings get --local com.actyx.os/general/displayName 10.2.3.23

# Get the settings of a specific app
$ ax settings get --local com.example.app1 10.2.3.23

#Get a specific setting from an app
$ ax settings get --local com.example.app1/setting1 10.2.3.23
```
