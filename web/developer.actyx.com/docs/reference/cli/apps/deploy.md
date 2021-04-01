---
title: ax apps deploy
---

<!-- TODO NKI: replace with correct link -->

:::warning `ax apps` subcommand is deprecated
Managing apps via the Actyx CLI is deprecated and will no longer be supported in future ActyxOS versions.
For more information on this, please refer to [this guide](../../how-to-guides/configuring-and-packaging/actyx-swarms.mdx)
:::

```text title="Deploy an app to an ActyxOS node"
USAGE:
    ax apps deploy [FLAGS] [OPTIONS] <PATH> <NODE>

FLAGS:
    -f, --force      Force update even if version number hasn't changed
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

OPTIONS:
        --arch <arch>    If you do not add the path to a file but to a
                         directory, ActyxOS will automatically package and
                         deploy the app that is in that directory. In case
                         your app can be packaged for more than one
                         architecture, please specify the architecture you want
                         to deploy to (`aarch64` or `x86_64`)

ARGS:
    <PATH>    Path to the app directory or tarball to process. Use `.` to use
              the current working directory
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

See the following example usages of the `ax apps deploy` command:

```text title="Example Usage"
# Deploy a packaged app tarball to a node
ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23
```
