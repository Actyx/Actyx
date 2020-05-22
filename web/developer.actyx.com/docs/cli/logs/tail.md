---
title: ax logs tail
---

## Access logs generated on a node

```bash
$ ax logs tail --help
USAGE: ax logs tail [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv      Increase verbosity
    -h, --help         Prints help information
    --local            Process over local network
    --all-entries      Get all entries, overrides `--entries`
    -f, --follow       Keep running and output entries as they are created
    -j or --json       Format output as JSON

OPTIONS:
    -n, --entries <num> Output NUM last entries [default: 20]

ARGS:
    <NODE>           Node ID or, if using `--local`, the IP address, of the
                     node to perform the operation on. You may also pass in a
                     file with a value using the syntax `@file.txt` or have the 
                     command one value from stdin using `@-`.
```

Please see the following usage examples for the `ax logs tail` command:

```bash
# Access the last 40 loggest entries from a node
$ ax logs tail --local -n 40 10.2.3.23

# Access logs in structured (json) format
$ ax logs tail --local --format json 10.2.3.23

# Follow logs as they are created
$ ax logs tail --local --follow 10.2.3.23
```