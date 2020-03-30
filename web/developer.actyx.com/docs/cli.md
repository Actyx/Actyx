---
title: Actyx CLI
---

The **Actyx Command Line Interface (CLI)** is a unified tool to manage your ActyxOS nodes and apps.

Both app developers and system administrators need tools to package, deploy, monitor and undeploy ActyxOS apps. The _Actyx CLI_ is a command-line tool that provides these capabilities through a number of commands.

**Interact with your nodes**
- List specified nodes with [`ax nodes ls`](#ax-nodes-ls)

**Build, validate, package, deploy, start and stop apps**
- List apps installed on a node with [`ax apps ls`](#ax-apps-ls)
- Validate the app you are building with [`ax apps validate`](#ax-apps-validate)
- Package apps for deployment with [`ax apps package`](#ax-apps-package)
- Deploy apps onto one or more nodes with [`ax apps deploy`](#ax-apps-deploy)
- Undeploy apps from one or mode nodes with [`ax apps undeploy`](#ax-apps-undeploy)
- Start apps on one or mode nodes with [`ax apps start`](#ax-apps-start)
- Stop apps on one or mode nodes with [`ax apps stop`](#ax-apps-stop)

**Get and set ActyxOS and app settings on nodes**
- Understand what settings scopes are available with [`ax settings scopes`](#ax-settings-scopes)
- Get settings schemas from a node with [`ax settings schema`](#ax-settings-schema)
- Get the current settings from a node with [`ax settings get`](#ax-settings-get)
- Set settings on nodes with [`ax settings set`](#ax-settings-set)
- Unset settings from nodes with [`ax settings unset`](#ax-settings-unset)

**Access logs from your nodes**
- Get historical logs or follow them with [`ax logs tail`](#ax-logs-tail)

**Create, configure and manage your swarms**
- Create a brand-new swarm key with [`ax swarms keygen`](#ax-swarms-keygen)

> Local-only for now
>
> As you will see in the examples below, the Actyx CLI currently only supports local communication with nodes, i.e. in the local-area network. Please use the `--local` flag until we release the [Actyx Console](/os/docs/actyx-console.html) and enable remote interactions between the Actyx CLI and ActyxOS nodes mediated by the Actyx Console.

# Nodes

### List your nodes and their status

```
$ ax nodes ls --help
USAGE: ax nodes ls [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv        Increase verbosity
    -h, --help           Prints help information
    --local              Process over local network
    --pretty             Prints a header in the text output 
    -j                   Shortcut for '--format json'

OPTIONS:
    --format FORMAT      Output list in the defined FORMAT (either `json` or 
                         `text`) [default: text]. The last occurrence of the 
                         format command or the shortcut wins.  

ARGS:
    <NODE>...            Node IDs or, if using `--local`, the IP addresses, of
                         the node(s) to perform the operation on. You may also
                         pass in a file with a value on each line using the syntax
                         `@file.txt` or have the command read one value per line
                         from stdin using `@-`.
```

> Output of `ax nodes ls`
>
> If the node is reachable, the output of `ax nodes ls` will show you its status. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:
> - Host unreachable
> - ActyxOS unreachable (this means the host was reachable but the TCP connection reset) 

See the following examples of using the `ax nodes ls` command:
```bash
# get the status of all specified nodes in the local network
$ ax nodes ls --pretty --local 10.2.3.23 10.2.3.24 10.2.3.25
NODE ID    DISPLAY NAME  STATE   SETTINGS LICENSE  APPS DEPLOYED APPS RUNNING  STARTED                    VERSION
10.2.3.23  MY NODE       running    valid   valid             23           17  2020-03-18T06:17:00+01:00  1.0.0
10.2.3.24  ActyxOS unreachable
10.2.3.25  Host unreachable

# get the status of all nodes in the local network as a json object
$ ax nodes ls --local --format json 10.2.3.23 10.2.3.24
{
    "reachable": [
        {
            "id": "10.2.3.23",
            "name": "MY NODE",
            "state": "running",
            "settings": "valid",
            "license": "valid", 
            "apps_deployed": 23,
            "apps_running": 17,
            "started_iso": "2020-03-18T06:17:00+01:00"
            "started_unix": 1584512220
            "version": "1.0.0"
        }
    ],
    "actyxos_unreachable": [
        {
            "id": "10.2.3.24"
        }
    ],
    "host_unreachable": [
        {
            "id": "10.2.3.25"
        }
    ]

}

```

> `ax nodes ls` only returns the state of the node
>
> Please keep in mind that `state`, `settings` and `license` in the  `ax nodes ls` command **only** refer to the node itself. If you want more detailed information about the state of the apps on a node, you need to use [`ax apps ls`](#ax-apps-ls).



# Apps

### Find out which apps are installed on your nodes

```
$ ax apps ls --help
USAGE: ax apps ls [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv        Increase verbosity
    -h, --help           Prints help information
    --local              Process over local network
    -j                   Shortcut for '--format json'

OPTIONS:
    --format FORMAT      Output list in the defined FORMAT (either `json` or 
                         `text`) [default: text]. The last occurrence of the 
                         format command or the shortcut wins.  


ARGS:
    <NODE>...            Node IDs or, if using `--local`, the IP addresses, of
                         the node(s) to perform the operation on. You may also
                         pass in a file with a value on each line using the syntax
                         `@file.txt` or have the command read one value per line
                         from stdin using `@-`.
```

> Output of `ax apps ls`
>
> If a node is reachable, the output of `ax apps ls` will list the status of all apps deployed on that node. If the node is unreachable, the output contains information why the node could not be reached. The Actyx CLI distinguishes 2 cases:
> - Host unreachable
> - ActyxOS unreachable (this means the host was reachable but the TCP connection reset) 


See the following examples of using the `ax apps ls` command:

```bash
# List the apps on two nodes in your local network
$ ax apps ls --pretty --local 10.2.3.23 10.2.3.24 10.2.3.25

NODE ID    APP ID         STATE    SETTINGS LICENSE  MODE      STARTED                    VERSION
10.2.3.23  com.actyx.mwl  running     valid   valid  enabled   2020-03-18T06:17:00+01:00  1.0.0
10.2.3.24  ActyxOS unreachable
10.2.3.25  Host unreachable

# get the status of apps on two nodes in the local network as a json object
$ ax apps ls --local --format 'json' 10.2.3.23 10.2.3.24
{
    "reachable": [
        {
            "nodeid": "10.2.3.23",
            "appid": "com.actyx.mwl"
            "state": "running",
            "settings": "valid",
            "license": "valid",
            "mode": "enabled",
            "started_iso": "2020-03-18T06:17:00+01:00"
            "started_unix": 1584512220
            "version": "1.0.0"
        }
    ],
    "actyxos_unreachable": [
        {
            "id": "10.2.3.24"
        }
    ],
    "host_unreachable": [
        {
            "id": "10.2.3.25"
        }
    ]
}

> `ax apps ls` only returns the state of the apps
>
> Please keep in mind that `state`, `settings` and `license` in the  `ax apps ls` command **only** refer to the apps deployed on a node. If you want more detailed information about the state of the node itself, you need to use [`ax nodes ls`](#ax-nodes-ls).

# Use an address in a file
$ ax apps ls --local @address.txt

# Pass the address from stdin
$ echo "10.2.3.23" | ax apps ls --local @-
```

### Validate apps and app manifests

```bash
$ ax apps validate --help
USAGE: ax apps validate [FLAGS] <PATH>...

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

Check out these examples showing common usages of the `ax apps validate` command:

```bash
# Validate the app in the current directory
$ ax apps validate

# Validate an app in a specific directory
$ ax apps validate myApp/

# Validate multiple apps in parallel
$ ax apps validate myApp1/ myApp2/ ../specialApp

# Validate a list of apps whose directories are in a file
$ ax apps validate @paths.txt
```

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

### Deploy apps to nodes

```bash
$ ax apps deploy --help
USAGE: ax apps deploy [FLAGS] <PATH> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    --force          Force update even if version number hasn't changed

ARGS:
    <PATH>           Path to the app directory or tarballs to process. Use
                     `.` for using the current working directory. You may also
                     pass in a file with a value on each line using the syntax
                     `@file.txt` or have the command read one value per line
                     from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

See the following example usages of the `ax apps deploy` command:

```bash
# Deploy the app in the current directory
$ ax apps deploy --local . 10.2.3.23

# Deploy a packaged tarball to a node
$ ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23

# Deploy an app to multiple nodes
$ ax apps deploy --local myApp1-1.0.0.tar.gz 10.2.3.23 10.2.3.24

# Deploy multiple apps to a node
$ echo "myApp1-1.0.0.tar.gz
myApp2-1.0.0.tar.gz" | ax apps deploy --local @- 10.2.3.23

# Deploy multiple apps to multiple nodes
$ cat apps.txt
myApp1-1.0.0.tar.gz
myApp2-1.0.0.tar.gz
$ cat nodes.txt
10.2.3.23
10.2.3.24
$ ax apps deploy --local @apps.txt @nodes.txt
```

### Undeploy apps from nodes

```bash
$ ax apps undeploy --help
USAGE: ax apps undeploy [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <APP>            IDs of the app(s) to undeploy from the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

Here are a couple of example of using the `ax apps undeploy` command:

```bash
# Undeploy a specific app from a node
$ ax apps undeploy --local com.example.myapp1 10.2.3.23

# Undeploy an app from multiple nodes
$ ax apps undeploy --local com.example.myapp1 10.2.3.23 10.2.3.24

# Undeploy multiple apps from a node
$ echo "com.example.myapp1
com.example.myapp2" | ax apps undeploy --local @- 10.2.3.23
```

### Start apps on nodes

```bash
$ ax apps start --help
USAGE: ax apps start [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <APP>            IDs of the app(s) to start on the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

> Some runtimes may not allow remote starting of apps
>
> Because of constraints imposed by the host platform, some runtimes may not be able to start apps automatically. In such cases, the host platform may require an end-user to start the app using platform-defined means such as a graphical user-interface control.

Here are a couple of example of using the `ax apps start` command:

```bash
# Start a single app on a single node
$ ax apps start --local com.example.app 10.2.3.23
com.example.app successfully started on 10.2.3.23

# Start multiple apps using stdin
$ echo "com.example.myapp1
com.example.myapp2" | ax apps start --local @- 10.2.3.23

# Start a single app that is already running
$ ax apps start --local com.example.app 10.2.3.23
com.example.app is already running on 10.2.3.23
```

### Stop apps on nodes

```bash
$ ax apps stop --help
USAGE: ax apps stop [FLAGS] <APP> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <APP>            IDs of the app(s) to stop on the given nodes. You may
                     also pass in a file with a value on each line using the
                     syntax `@file.txt` or have the command read one value per
                     line from stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

Here are a couple of example of using the `ax apps stop` command:

```bash
# Stop a single app on a single node
$ ax apps stop --local com.example.app 10.2.3.23

# Stop multiple apps using stdin
$ echo "com.example.myapp1
com.example.myapp2" | ax apps stop --local @- 10.2.3.23
```

# Settings

## Get setting scopes from a node

```bash
$ ax settings scopes --help
USAGE: ax settings scopes [FLAGS] <NODE>

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <NODE>           Node ID or, if using `--local`, the IP address, of the node
                     to perform the operation on. You may also pass in a file with
                     a value on the first line using the syntax `@file.txt` or have
                     the command read one value per line from stdin using `@-`.
```

Here is a simple example of using the `ax settings scopes` command:

```bash
# Get the settings scopes from a node:
$ ax settings scopes --local 10.2.3.23
```

## Get setting schemas from a node

```bash
$ ax settings schema --help
USAGE: ax settings schema [FLAGS] <SCOPE> <NODE>

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <SCOPE>          Scope at which you want to get the settings.
    <NODE>           Node ID or, if using `--local`, the IP address, of the node
                     to perform the operation on. You may also pass in a file with
                     a value on the first line using the syntax `@file.txt` or have
                     the command read one value per line from stdin using `@-`.
```

Here is a simple example of using the `ax settings schema` command:

```bash
# Get the ActyxOS nodes settings schema from a node
$ ax settings schema --local ax.os 10.2.3.23

# Get the settings schema for a specific app from a node
$ ax settings schema --local com.example.app 10.2.3.23
```

## Get settings from a node

```bash
$ ax settings get --help
USAGE: ax settings get [FLAGS] <SCOPE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network
    --no-defaults    Do not return non-user-defined defaults from the schema

ARGS:
    <SCOPE>          Scope at which you want to get the settings.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

> Null or schema-conformant
>
> The return value of this command will always be either null or schema-conformant, unless the `--no-defaults` flag is used.

Here are a couple of examples of using the `ax settings get` command:

```bash
# Get the ActyxOS settings from a node:
$ ax settings get --local ax.os 10.2.3.23

# Get the display name set for a node
$ ax settings get --local ax.os/General/DisplayName 10.2.3.23

# Get the settings of a specific app
$ ax settings get --local com.example.app1 10.2.3.23

#Get a specific setting from an app
$ ax settings get --local com.example.app1/Setting1 10.2.3.23
```

## Set settings on one or more nodes

```bash
$ ax settings set --help
USAGE: ax settings set [FLAGS] <SCOPE> <VALUE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <SCOPE>          Scope at which you want to set the given settings.
    <VALUE>          The value you want to set at the given scope as a YAML
                     or JSON string. You may also pass in a file using the
                     syntax `@file.yml` or have the command read from
                     stdin using `@-`.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

> App must be stopped in order to set settings for it
>
> Nodes will only accept new settings if the relevant app is not running. For example: setting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).

> All app must be stopped in order to set node settings
>
> Nodes will only accept new settings for the `ax.os` scope if all apps on the node(s) are not running.

Please see the following usage examples for the `ax settings set` command:

```bash
# Setting for a node from a file
$ ax settings set --local ax.os @Node1.settings.yml 10.2.3.23

# Setting a single specific ActyxOS node setting
$ ax settings set --local ax.os/General/DisplayName "Node 1" 10.2.3.23

# Setting settings for multiple nodes at the same time
$ ax settings set --local ax.os/General/SwarmKey @swarm.key 10.2.3.23 10.2.3.24

# Setting ActyxOS settings for a node using stdin
$ echo "
General:
  SwarmKey: 4245c0e542a4f89985a92de178d2169dc7f3596a382828aa8381bc13370e9880
  DisplayName: My Node
  BootstrapNodes:
    - /ipfs/10.24.24.2
    - /ipfs/10.24.24.3" | ax settings set --local ax.os @- 10.2.3.23
```

## Unset settings on one or more nodes

```bash
$ ax settings unset --help
USAGE: ax settings unset [FLAGS] <SCOPE> <NODE>...

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <SCOPE>          Scope at which you want to set the given settings.
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

> App must be stopped in order to unset settings for it
>
> Nodes will only unset an app's settings if the relevant app is not running. For example: unsetting settings for the scope `com.example.app` will only work if the app with ID `com.example.app` is not running on the node(s).

> All app must be stopped in order to unset node settings
>
> Nodes will only unset settings for the `ax.os` scope if all apps on the node(s) are not running.

Please see the following usage examples for the `ax settings unset` command:

```bash
# Unset ActyxOS settings from a node
$ ax settings unset --local ax.os 10.2.3.23

# Unset a specific app's settings from a node
$ ax settings unset --local com.example.app 10.2.3.23

# Unset settings from multiple nodes
$ ax settings unset --local com.example.app 10.2.3.23 10.2.3.24

# Unset settings from multiple nodes defined in a file
$ ax settings unset --local com.example.app @nodes.txt
```

# Logs

## Access logs generated on a node

```bash
$ ax logs tail --help
USAGE: ax logs tail [FLAGS] [OPTIONS] <NODE>...

FLAGS:
    -v, -vv, -vvv      Increase verbosity
    -h, --help         Prints help information
    --local            Process over local network
    --all-entries      Get all entries, overrides `--entries`
    -f, --follow       Keep running and output entries as they are created

OPTIONS:
    -n, --entries NUM  Output NUM last entries [default: 20]
    --format FORMAT    Output log messages in the defined FORMAT (either
                       `json` or `text`) [default: text]

ARGS:
    <NODE>...        Node IDs or, if using `--local`, the IP addresses, of the
                     node(s) to perform the operation on. You may also pass in a
                     file with a value on each line using the syntax `@file.txt`
                     or have the command read one value per line from stdin
                     using `@-`.
```

Please see the following usage examples for the `ax logs tail` command:

```bash
# Access the last 40 loggest entries from a node
$ ax logs tail --local -n 40 10.2.3.23

# Access logs in structured (json) format
$ ax logs tail --local --format json 10.2.3.23

# Follow logs as they are created
$ ax logs tail --local --follow 10.2.3.23
```

# Swarms

## Create a swarm key

```bash
$ ax swarms keygen --help
USAGE: ax swarms keygen [FLAGS]

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
```

This command is extremely simple; see for yourself:

```bash
# Create a swarm key
$ ax swarms keygen

# Create a swarm key, save it and set it on a node
$ ax swarms keygen | tee swarm.key | ax settings set --local ax.os.General.SwarmKey @- 10.2.3.23
```