---
title: ax apps ls
---

### Get the states of the apps deployed to your nodes

```
$ ax apps ls --help
USAGE: ax apps ls [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv        Increase verbosity
    -h, --help           Prints help information
    --local              Process over local network
    -j                   Shortcut for '--format json'

OPTIONS:
    --format FORMAT      Output list in the defined FORMAT (either `json` or 
                         `text`) [default: text]. The last occurrence of the 
                         format command or the shortcut wins.  


ARGS:
    <NODE>...            Node IDs or, if using `--local`, the IP addresses, of
                         the node(s) to perform the operation on. You may also
                         pass in a file with a value on each line using the syntax
                         `@file.txt` or have the command read one value per line
                         from stdin using `@-`.
```

:::tip Output
If a node is reachable, the output of `ax apps ls` will list the status of all apps deployed on that node. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:
- Host unreachable
- ActyxOS unreachable (this means the host was reachable but the TCP connection reset) 
:::


See the following examples of using the `ax apps ls` command:

```bash
# List the apps on two nodes in your local network
$ ax apps ls --local 10.2.3.23 10.2.3.24 10.2.3.25

NODE ID    APP ID         STATE    SETTINGS LICENSE  MODE      STARTED                    VERSION
10.2.3.23  com.actyx.mwl  running     valid   valid  enabled   2020-03-18T06:17:00+01:00  1.0.0
10.2.3.24  ActyxOS unreachable
10.2.3.25  Host unreachable

# get the status of apps on two nodes in the local network as a json object
$ ax apps ls --local --format 'json' 10.2.3.23 10.2.3.24
{
    "reachable": [
        {
            "nodeid": "10.2.3.23",
            "appid": "com.actyx.mwl"
            "state": "running",
            "settings": "valid",
            "license": "valid",
            "mode": "enabled",
            "started_iso": "2020-03-18T06:17:00+01:00"
            "started_unix": 1584512220
            "version": "1.0.0"
        }
    ],
    "actyxos_unreachable": [
        {
            "id": "10.2.3.24"
        }
    ],
    "host_unreachable": [
        {
            "id": "10.2.3.25"
        }
    ]
}

# Use an address in a file
$ ax apps ls --local @address.txt

# Pass the address from stdin
$ echo "10.2.3.23" | ax apps ls --local @-

````

:::info`ax apps ls` only returns the state of the apps

Please keep in mind that **state**, **settings** and **license** in the  `ax apps ls` command **only** refer to the apps deployed on a node. If you want more detailed information about the node itself, you need to use [`ax nodes ls`](../nodes/ls).
:::

### Validate apps and app manifests

```bash
$ ax apps validate --help
USAGE: ax apps validate [FLAGS] <PATH>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information

ARGS:
    <PATH>...        Paths to the app manifests to process. If no path
                     is given, try to use the file `ax-manifest.yml` in
                     the current directory. You may also pass in a file
                     with a path on each line using the syntax `@paths.txt`
                     or have the command read one path per line from stdin
                     using `@-`.
```

Check out these examples showing common usages of the `ax apps validate` command:

```bash
# Validate the app in the current directory
$ ax apps validate

# Validate an app in a specific directory
$ ax apps validate myApp/

# Validate multiple apps in parallel
$ ax apps validate myApp1/ myApp2/ ../specialApp

# Validate a list of apps whose directories are in a file
$ ax apps validate @paths.txt
```
