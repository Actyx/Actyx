---
title: ax apps package
---

<!-- TODO NKI: replace with correct link -->

:::warning `ax apps` subcommand is deprecated
Managing apps via the Actyx CLI is deprecated and will no longer be supported in future ActyxOS versions.
:::

```text title="Package an app into a tarball for deployment"
USAGE:
    ax apps package [FLAGS] [PATH]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <PATH>    Path to the app manifest of the app to package or the directory
              that contains it [default: ax-manifest.yml]
```

The `ax apps package` command will write the packaged app as a tarball to disk, which can then be deployed.

Here are a couple of example uses of the `ax apps package` command:

```text title="Example Usage"
# Package an app in the current directory with default manifest ax-manifest.yml
ax apps package

# Package an app in the current directory with manifest another-manifest.yml
ax apps package another-manifest.yml

# Package the app in the specified directory myApp with default manifest
# ax-manifest.yml
ax apps package myApp/

# Package an app in the specified directory myApp with manifest
# myApp/another-manifest.yml
ax apps package myApp/another-manifest.yml
```

:::note Want to save the package somewhere else than the app directory?
The `ax apps package` command always stores its output in the directory from which it was called. If you would like to save the output somewhere else, you should call the Actyx CLI from that directory and provide it the path to the directory containing the app manifest or the path to the actual manifest. Here is an example:

```text title="Example Usage"
cd ../../../apps-packages
ax apps package ../projects/actyx-os/apps/example-app-1
```

:::
