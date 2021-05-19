---
title: ax logs tail
hide_table_of_contents: true
---

```text title="Get logs from a node"
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

```text title="Example Usage"
# Get the last 40 log entries from node at 10.2.3.23
ax logs tail --local -n 40 10.2.3.23

# Get the logs in structured JSON format
ax --json logs tail --local 10.2.3.23

# Keep running and output log entries as they are created
ax logs tail --local --follow 10.2.3.23
```
