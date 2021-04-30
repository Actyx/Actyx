---
title: ax apps validate
---

<!-- TODO NKI: replace with correct link -->

:::warning `ax apps` subcommand is deprecated
Managing apps via the Actyx CLI is deprecated and will no longer be supported in future ActyxOS versions.
:::

```text title="Validate an app manifest"
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

```text title="Example Usage"
# Validate an app in the current directory with default manifest ax-manifest.yml
ax apps validate

# Validate an app in the current directory with manifest another-manifest.yml
ax apps validate another-manifest.yml

# Validate an app in the specified directory myApp with manifest myApp/another-manifest.yml
ax apps validate myApp/another-manifest.yml

# Validate multiple apps in the specified directories with default manifest ax-manifest.yml
ax apps validate myApp1/ myApp2/ ../specialApp
```

import { NPS } from '../../../../src/components/NPS'

<NPS />
