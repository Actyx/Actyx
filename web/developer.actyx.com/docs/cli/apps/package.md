---
title: ax apps package
---

### Package apps for deployment

```bash
$ ax apps package --help
USAGE: ax apps package [FLAGS] <PATH>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    -j or --json     Format output as JSON


ARGS:
    <PATH>...        Paths to the app manifests to process. If no path
                     is given, try to use the file `ax-manifest.yml` in
                     the current directory. You may also pass in a file
                     with a path on each line using the syntax `@paths.txt`
                     or have the command read one path per line from stdin
                     using `@-`.
```

Here are a couple of example uses of the `ax apps package` command:

```bash
# Package the app in the current directory
$ ax apps package

# Package an app in a specific directory
$ ax apps package myApp/

# Package multiple apps in parallel
$ ax apps package myApp1/ myApp2/ ../specialApp

# Package a list of apps whose directories are in a file
$ ax apps package @paths.txt
```

:::note Want to save the package somewhere else than the app directory?
The `ax apps package` command always stores its output in the directory from which it was called. If you would like to save the output somewhere else, you should call the Actyx CLI from that directory and provide it the path to the directory containing the app manifest or the path to the actual manifest. Here is an example:
```
cd ../../../apps-packages
ax apps package ../projects/actyx-os/apps/example-app-1
```
:::
