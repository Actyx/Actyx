---
title: ax events restore
---

```text title="Restoring events from a dump"
USAGE:
    ax events restore [FLAGS] [OPTIONS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -q, --quiet      suppress progress information on stderr
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
        --cloud <FILE>       load dump via the cloud and store it as the given filename
    -i, --identity <FILE>    File from which the identity (private key) for authentication is read
    -I, --input <FILE>       file to read the dump from

ARGS:
    <NODE>    the IP address or <host>:<admin port> of the node to perform the operation on
```

This command can be used to read a dump file and modify an Actyx node to create a topic store from it.
All events from the dump will be written into that store, preserving the `isLocal` status of events as it was on the source node where the dump was taken:
an event that was local to the source node will also be local on the target node.

:::warning This is not a backup mechanism!
It is important to note that this event restore mechanism is only intended for debugging purposes, taking events from one environment into another.
Restoring events on a production node is not recommended, in particular you must not attempt to bring the restored events back onto the production swarm topic — this will lead to event duplication and other errors.
:::

In order to avoid mistakes, `ax events restore` will switch the target Actyx node into read-only mode and onto a new topic named after the dump’s timestamp.

```text Example
$ ax events restore -I dump localhost
received 60617 bytes
sending dump from node pWEd7zANCPqdmERpP.5UhThNRNEI9Hv85L2BS60NrSY topic `default-topic`
uploading to topic `dump-2021-12-09T09:29:34.615126037+01:00`
in total 60617 bytes uploaded
topic switched to `dump-2021-12-09T09:29:34.615126037+01:00`
Actyx node switched into read-only network mode
```

Now you can run your application logic against the target Actyx node and see exactly what went on in the production environment.
When you are done investigating, you can switch your Actyx node back onto the topic it was using before (and possibly enable read-write mode as well).
Afterwards you may remove the dump’s files from the `actyx-data/store` folder (be sure to check the exact timestamp against the output from the restore operation.)

:::tip How to reset the dump topic
If you want to reset the dump topic to its original state after playing with your application logic a bit, you can just reimport the same dump; this will first erase the database files and then write them afresh.
:::

## Cloud Transfer

It can be inconvenient to move files between production machines and the test or development environment.
If both sides of this transfer can access HTTPS in the cloud, you can use the Actyx cloud mirror to transfer the dump.

```text title="Transfer via the cloud"
$ ax events restore --cloud dump localhost
connection open, waiting for dump
now is a good time to start `ax events dump --cloud 75012105-8538-4b85-8147-9df57f59c789` on the source machine
```

The event dump will be uploaded to the Actyx node (at `localhost` in this case) as well as stored into the given dump file (named `dump` here).
This allows you to reset the topic later as described above.

On the sending side, include the option `--cloud <TOKEN>` in your `ax events dump` call and make sure to not specify an output file.
Then the rest of the process will work exactly as with other transfer methods.
