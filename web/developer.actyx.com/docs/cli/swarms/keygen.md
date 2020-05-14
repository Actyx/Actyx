---
title: ax swarms keygen
---

## Create a swarm key

```bash
$ ax swarms keygen --help
USAGE: ax swarms keygen [FLAGS]

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
```

This command is extremely simple; see for yourself:

```bash
# Create a swarm key
$ ax swarms keygen

# Create a swarm key, save it and set it on a node
$ ax swarms keygen | tee swarm.key | ax settings set --local com.actyx.os/general/swarmKey @- 10.2.3.23
```