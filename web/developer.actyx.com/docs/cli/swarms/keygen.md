---
title: ax swarms keygen
---

<!-- markdownlint-disable-file MD040 -->

## Generate a new ActyxOS swarm key

```
USAGE:
    ax swarms keygen [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -o, --output <output>    Create file <output> and write the generated key
                             to it
```

This command is extremely simple; see for yourself:

```
# Create a swarm key
ax swarms keygen

# Create a swarm key, save it and set it on a node
ax swarms keygen | tee swarm.key | ax settings set --local com.actyx.os/general/swarmKey @- 10.2.3.23
```
