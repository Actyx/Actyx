---
title: ax topics ls
hide_table_of_contents: true
---

```text title="Show topic info"
USAGE:
    ax topics ls [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v

OPTIONS:
    -i, --identity <identity>    The private key file to use for authentication
    -t, --timeout <timeout>      Timeout time for the operation (in seconds, with a maximum of 255) [default: 5]

ARGS:
    <NODE>...    The IP addresses or <host>:<admin port> of the target nodes
```

If the node is unreachable, it is displayed as such in the output.
See the following examples of using the `ax topics ls` command:

```text title="Example usage"
$ ax topics ls localhost
┌─────────────────────────────────────────────┬───────────┬───────────────┬─────────┬────────┐
│ NODE ID                                     │ HOST      │ TOPIC         │ SIZE    │ ACTIVE │
├─────────────────────────────────────────────┼───────────┼───────────────┼─────────┼────────┤
│ 6WrYWzS/WBsTwXMmMn9ZTHf4zmRTWyPHIoNQuzl7tKY │ localhost │ default-topic │ 147456  │        │
│                                             │           │ old-topic     │ 380928  │        │
│                                             │           │ running-topic │ 6627632 │ *      │
└─────────────────────────────────────────────┴───────────┴───────────────┴─────────┴────────┘

# Get the topic information as a JSON object
$ ax topics ls localhost --json | jq
{
  "code": "OK",
  "result": [
    {
      "connection": "Reachable",
      "host": "localhost",
      "response": {
        "nodeId": "6WrYWzS/WBsTwXMmMn9ZTHf4zmRTWyPHIoNQuzl7tKY",
        "activeTopic": "running-topic",
        "topics": [
          {
            "default-topic": 147456
          },
          {
            "old-topic": 380928
          },
          {
            "running-topic": 6627632
          }
        ]
      }
    }
  ]
}
```
