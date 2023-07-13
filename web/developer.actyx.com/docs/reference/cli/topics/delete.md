---
title: ax topics delete
hide_table_of_contents: true
---

```text title="Delete a given topic"
USAGE:
    ax topics delete [FLAGS] [OPTIONS] <topic> <NODE>...

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v

OPTIONS:
    -i, --identity <identity>    The private key file to use for authentication
    -t, --timeout <timeout>      Timeout time for the operation (in seconds, with a maximum of 255) [default: 5]

ARGS:
    <topic>      The topic to delete
    <NODE>...    The IP addresses or <host>:<admin port> of the target nodes
```

The delete operation does not fail if the topic does not exist, instead,
the `DELETED` field will be set to `Y` if files were deleted.
See the following example of using the `ax topics delete` command:

```text title="Example usage"
$ ax topics delete old-topic localhost
┌─────────────────────────────────────────────┬───────────┬─────────┐
│ NODE ID                                     │ HOST      │ DELETED │
├─────────────────────────────────────────────┼───────────┼─────────┤
│ 6WrYWzS/WBsTwXMmMn9ZTHf4zmRTWyPHIoNQuzl7tKY │ localhost │ Y       │
└─────────────────────────────────────────────┴───────────┴─────────┘
```
