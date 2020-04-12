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

OPTIONS:
    -n, --entries NUM  Output NUM last entries [default: 20]
    --format FORMAT    Output log messages in the defined FORMAT (either
                       `json` or `text`) [default: text]

ARGS:
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
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