---
title: ax apps validate
---

### Validate apps and app manifests

```bash
ax apps validate --help
USAGE: ax apps validate [FLAGS] <PATH>...

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

Check out these examples showing common usages of the `ax apps validate` command:

```bash
# Validate the app in the current directory
ax apps validate

# Validate an app in a specific directory
ax apps validate myApp/

# Validate multiple apps in parallel
ax apps validate myApp1/ myApp2/ ../specialApp

# Validate a list of apps whose directories are in a file
ax apps validate @paths.txt
```
