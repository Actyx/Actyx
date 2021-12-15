---
title: ax events dump
---

```text title="Dumping events from a node"
USAGE:
    ax events dump [FLAGS] [OPTIONS] <QUERY> <NODE>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -q, --quiet      suppress progress information on stderr
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
        --cloud <TOKEN>      send dump via the cloud (start restore first to get the token)
    -i, --identity <FILE>    File from which the identity (private key) for authentication is read
    -o, --output <FILE>      file to write the dump to

ARGS:
    <QUERY>    selection of event data to include in the dump
    <NODE>     the IP address or <host>:<admin port> of the node to perform the operation on
```

An event dump is a compressed file that contains information on the source Actyx node and a selection of events.
The events are selected using the [AQL query](../../aql.mdx) given as an argument, you can filter in any way you like.
The purpose of an event dump is to take the selected event history from a production environment and bring it into testing or development, so that application logic can be examined (e.g. to identify bugs).

```text title="Exporting all events for a given app"
$ ax events dump -o out 'FROM appId(com.example.my-app)' localhost
connected to localhost [127.0.0.1:4458]
info block written
892 events written (maximum size was 4303)
```

The `out` file can now be copied to a development machine and examined using [`ax events restore`](./restore.md).
You can also write the dump to standard output (by leaving out the `-o` option) or send it via the cloud — see [restoring via the cloud](./restore.md#cloud-transfer).
