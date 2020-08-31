---
title: ax apps package
---

<!-- markdownlint-disable-file MD040 -->

### Package an app

```
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

Here are a couple of example uses of the `ax apps package` command:

```
# Package the app in the current directory
ax apps package

# Package an app in a specific directory
ax apps package myApp/

# Package multiple apps in parallel
ax apps package myApp1/ myApp2/ ../specialApp

# Package a list of apps whose directories are in a file
ax apps package @paths.txt
```

:::note Want to save the package somewhere else than the app directory?
The `ax apps package` command always stores its output in the directory from which it was called. If you would like to save the output somewhere else, you should call the Actyx CLI from that directory and provide it the path to the directory containing the app manifest or the path to the actual manifest. Here is an example:

```
cd ../../../apps-packages
ax apps package ../projects/actyx-os/apps/example-app-1
```

:::
