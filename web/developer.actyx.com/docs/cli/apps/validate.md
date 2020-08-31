---
title: ax apps validate
---

### Validate app manifests

```
USAGE:
    ax apps validate [FLAGS] [PATH]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <PATH>...    Path to the app manifest to validate or the directory that
                 contains it [default: ax-manifest.yml]
```

Check out these examples showing common usages of the `ax apps validate` command:

```
# Validate the app in the current directory
ax apps validate

# Validate an app in a specific directory
ax apps validate myApp/

# Validate multiple apps in parallel
ax apps validate myApp1/ myApp2/ ../specialApp

# Validate a list of apps whose directories are in a file
ax apps validate @paths.txt
```
