---
title: ax logs tail
---

## Get logs from a node

```
USAGE:
    ax logs tail [FLAGS] [OPTIONS] <NODE>

FLAGS:
        --all-entries    Get all log entries (overrides --entries)
    -f, --follow         Keep running and output entries as they are created
    -h, --help           Prints help information
    -l, --local          Process over local network
    -V, --version        Prints version information
    -v                   Verbosity level. Add more v for higher verbosity
                         (-v, -vv, -vvv, etc.)

OPTIONS:
    -n, --entries <entries>    Output the last <entries> entries [default: 20]

ARGS:
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Please see the following usage examples for the `ax logs tail` command:

```
# Access the last 40 loggest entries from a node
ax logs tail --local -n 40 10.2.3.23

# Access logs in structured (json) format
ax logs tail --local --format json 10.2.3.23

# Follow logs as they are created
ax logs tail --local --follow 10.2.3.23
```
