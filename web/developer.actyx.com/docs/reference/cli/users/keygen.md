---
title: ax users keygen
---

```text title="Generate a new user key pair for interacting with an Actyx node"
USAGE:
    ax users keygen [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -j, --json       Format output as JSON
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
    -o, --output <output>    Path in which to save the private key. The public key will be generated in the same
                             directory with the `.pub` suffix
```

Please see the following usage examples for the `ax users keygen` command:

```text title="Example Usage"
# Generate a new user keys in default location
ax1 users keygen
Generating public/private key pair ..
Enter path in which to save the key (<OS-specific default path>/actyx/keys/users): 
Your private key has been saved at <OS-specific default path>/actyx/keys/users/id
Your public key has been saved at <OS-specific default path>/actyx/keys/users/id.pub
The key's fingerprint is: 0WBOOWqi2Ub5SPi5btKN5H5BzFPcjyULwUKUN2dWVsMI=

# Generate a new swarm key, create file swarm.key and write the generated key to it
ax users keygen -o mykeys
Generating public/private key pair ..
Your private key has been saved at mykeys
Your public key has been saved at mykeys.pub
The key's fingerprint is: 09a36bJYYYSusZuQoum6x2zgqtHxYP31ov0RHRWIzwVo=
```