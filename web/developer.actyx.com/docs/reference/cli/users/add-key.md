---
title: ax users add-key
---

```text title="Add a key to a locally running Actyx node"
USAGE:
    ax users add-key [FLAGS] [OPTIONS] <PATH>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.)

OPTIONS:
    -i, --identity <FILE>    File from which the identity (private key) for authentication is read

ARGS:
    <PATH>    Path to the `actyx-data` folder you wish to modify
```

This command uses local filesystem access to the Actyx data folder to modify the node database and insert the given key into the authorized keys list.
For this operation to succeed, the Actyx node must not be running and you must have write access to the database files.

If you try to add your key even though it is already present, you’ll see such an error:

```text title="Adding a key again"
$ ax users add-key actyx-data
locking actyx-data/lockfile
locked LockFile { locked: true, id: FileId, desc: 9 }
[ERR_SETTINGS_INVALID] Error: Validation failed.
        Errors:
                /admin/authorizedUsers: UniqueItems condition is not met.
```
