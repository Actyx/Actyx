---
title: ax nodes ls
---

### Get important information on your nodes

```
$ ax nodes ls --help
USAGE: ax nodes ls [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv        Increase verbosity
    -h, --help           Prints help information
    --local              Process over local network
    --pretty             Prints a header in the text output 
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

:::tip Output of `ax nodes ls`

If the node is reachable, the output of `ax nodes ls` will show you its status. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:
- Host unreachable
- ActyxOS unreachable (this means the host was reachable but the TCP connection reset) 
:::

See the following examples of using the `ax nodes ls` command:
```bash
# get the status of all specified nodes in the local network
$ ax nodes ls --pretty --local 10.2.3.23 10.2.3.24 10.2.3.25
NODE ID    DISPLAY NAME  STATE   SETTINGS LICENSE  APPS DEPLOYED APPS RUNNING  STARTED                    VERSION
10.2.3.23  MY NODE       running    valid   valid             23           17  2020-03-18T06:17:00+01:00  1.0.0
10.2.3.24  ActyxOS unreachable
10.2.3.25  Host unreachable

# get the status of all nodes in the local network as a json object
$ ax nodes ls --local --format json 10.2.3.23 10.2.3.24
{
    "reachable": [
        {
            "id": "10.2.3.23",
            "name": "MY NODE",
            "state": "running",
            "settings": "valid",
            "license": "valid", 
            "apps_deployed": 23,
            "apps_running": 17,
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

```

:::info `ax nodes ls` only returns the state of the node

Please keep in mind that **state**, **settings** and **license** in the  `ax nodes ls` command **only** refer to the node itself. If you want more detailed information about the state of the apps on a node, you need to use [`ax apps ls`](#apps-ls).
:::

