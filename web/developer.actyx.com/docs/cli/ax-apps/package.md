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
