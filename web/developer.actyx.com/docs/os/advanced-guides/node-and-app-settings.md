---
title: Node and App Settings
---

Configuring the behavior of ActyxOS nodes and apps.

Throughout their lifecycle you may want to configure the behavior of nodes and apps. ActyxOS allows you to do this with the concept of _settings_.

Key capabilities:

- Efficient definition of node and app settings
- Safe and granular deployment of node and app settings
- Simple method for defining app settings schemas and defaults

## Basics

### Settings and schemas

Settings are a means to configure the behavior of systems. Depending on the settings, the system will behave differently. Which parts of a system are configurable and in which fashion is defined by the developer of the system. Throughout the development phase she may choose to make certain behaviors configurable by defining settings that a user of the system can later _set_ to arbitrary or well-defined values.

Consider, as a simple example, the language shown in a user-interface. The developer of said interface can decide to only and always show UI element in English, or she can make it configurable through a _language_ setting. The setting may be defined as follows:

| Setting          | Type     | Permitted values                      | Default value |
|------------------|----------|---------------------------------------|---------------|
| Language         | `string` | `"english"`, `"french"` or `"german"` | `"english"`   |

How you can configure the behavior of ActyxOS nodes&mdash;node settings&mdash;has been defined by us. How you can configure an app running on ActyxOS has been defined by the app developer. This definition is done in the form of _settings schemas_ and in the case of ActyxOS, specifically using [JSON Schema](https://json-schema.org/). Taking the example above, the developer would have defined a settings schema for the app as follows:

```json
{
  "$schema": "http://json-schema.org/draft-06/schema#",
  "type": "object",
  "properties": {
    "language": {
      "type": "string",
      "enum": ["english", "french", "german"],
      "default": "english"
    }
  }
}
```

For most apps, the settings schemas will be a lot more involved than this simple example. Indeed, settings are often not even flat lists of key-value pairs, but rather complete trees. An SAP Connector app might for example actually have a structure as follows:

```yaml
com.example.sap_connector: # Root of the settings tree
  ui:
    language: string # ("english", "french" or "german")
    fontSize: number
  connectivity:
    sap_endpoint:
      ip_address: string
      port: number
    sap_authentication:
      username: string
      password: string
# etc...
```

Now when you combine the settings of this app, with the settings of the first example app and the node settings we have defined, you come to the so-called _settings object_ of any ActyxOS node. It has the following structure:

```
com.actyx.os:
  # ActyxOS node settings
  # ...
com.example.app1:
  # Settings for App 1
  # ...
com.example.app2:
  # Settings for App 2
  # ...
# etc...
```

The settings object and, more importantly, the settings schemas are a very powerful construct because they

- specify _exactly_ what can be configured and how,
- allow ActyxOS to verify the correctness of settings,
- provide the app developer with valuable guarantees; and, thus,
- help avoid critical failures in production systems.

This was just a short introduction and touched only on the basics of settings and schemas. We will dive into more depth in the concrete usage cases.

### Node settings

We have defined exactly how the behavior of ActyxOS nodes can be configured in our ActyxOS [_Node Settings Schema_](../api/node-settings-schema.md) which you can download anytime from [here](/schemas/os/node-settings.schema.json).

Here are a couple of the most important ActyxOS nodes settings:

| Setting          | Type     | Permitted values                      | Default value |
|------------------|----------|---------------------------------------|---------------|
| Display name     | `string` | _any string_                          | ""            |
| Swarm key        | `string` | _a string with exactly 64 characters_ | ""            |
| Swarm topic      | `string` | _any string_                          | ""            |

To check out the complete set of settings, download the _Node Settings Schema_ linked to above.

### App settings

As an app developer it is completely up to you what you want users of your app to be able to configure. As you will see below, you will do so by defining your own _App Settings Schema_ using [JSON Schema](https://json-schema.org/).

## Usage

In this section we will go through the three main areas where node and app settings are used:

1. when [configuring nodes](#configuring-nodes)
1. when [developing apps](#developing-apps)
1. when [configuring apps](#configuring-apps)

### Configuring nodes

ActyxOS provides a number of settings that you can set. Some of those are required for the node to work, whereas others are optional. You can download the full ActyxOS [_Node Settings Schema_](../api/node-settings-schema.md) [here](/schemas/os/node-settings.schema.json). In this section we will show you how you can configure a node.

The primary tool for setting settings, both at the node and the app level, is the [Actyx CLI](../../cli.md). The Actyx CLI provides three important commands for doing so:

- `ax settings scopes` for figuring out what the top-level _scopes_ of the _settings object_ on the node are,
- `ax settings get` to get settings from a node; and,
- `ax settings set` to set settings on a node.

Let's jump into an example, where we want to configure a brand-new ActyxOS node. First we create a new file&mdash;let's call it `node-settings.yml` and set all the settings to the values we want:

```yml
general:
  displayName: My Test Node
  swarmKey: L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK
  bootstrapNodes:
    - /dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH
licensing:
  os: development
  apps: {}
services:
  consoleService: {}
  eventService:
    readOnly: false
    topic: SampleTopic
  dockerRuntime: {}
  webViewRuntime: {}
  ```

Now we need to set these settings on the node (which, in this example, is reachable at 10.2.3.23) using the Actyx CLI's `ax settings set` command:

```bash
# Set the settings defined in `node-settings.yml` on the node
$ ax settings set --local com.actyx.os @node-settings.yml 10.2.3.23
#             ^           ^      ^                 
#             | set       |      | read from the given file
#                         |
#                         | set the settings at the `com.actyx.os` scope
```

If we wanted to find out if there are any top-level settings scopes other than `com.actyx.os`, the pre-defined scope at which you configure the node itself, we could use the Actyx CLI's `ax settings scopes` command:

```bash
# Get top-level scopes on the node
$ ax settings scopes --local 10.2.3.23
com.actyx.os
```

What if you want to change a single one of the settings? You could, of course, edit the file and run through the same process again. The Actyx CLI offers a much simpler way of doing this though. Check out how we could, for example, just change the ActyxOS [_Event Service_](../api/event-service.md) topic:

```bash
# Change a setting in the tree
$ ax settings set --local com.actyx.os/Services/EventService/Topic "New Topic" 10.2.3.23
#                         ^    ^                            ^
#                         |    |                            | value to set the setting to
#                         |    |
#                         |    | path into the settings object
#                         |
#                         | top-level scope as the entry point
```

The Actyx CLI allows you to not only set settings at top-level scopes such as `com.actyx.os`, but rather allows you to change leafs or even sub-trees in the node's settings object.

### Developing apps

As an app developer you can precisely define what system administrators deploying and operating your app may configure. You can also, of course, access the settings that have been set for your app from within your app.

#### Defining an app settings schema

Settings schemas are defined using the [JSON Schema](https://json-schema.org/) up to **Draft-06**. Consider for instance, that you would like to offer two settings in your app:

1. a _time unit_ without a default value (making it a required setting)
1. a _background color_ with a default value (making it an optional setting)

You could do so by writing the following settings schema:

```json
{
  "$schema": "http://json-schema.org/draft-06/schema#",
  "type": "object",
  "properties": {
    "timeUnit": {
      "type": "string"
    },
    "backgroundColor": {
      "type": "string",
      "enum": ["green", "black", "blue", "red"],
      "default": "green"
    },
  },
  "required": [
    "timeUnit"
  ]
}
```

Following association of this schema with your app, ActyxOS will now ensure that only settings meeting this schema will ever be provided to your app.

> What if the settings are invalid?
>
> ActyxOS will only ever start an app if the settings on the node have been validated against the revelant app settings schema. Otherwise the app will be in a special state called _stopped (misconfigured)_.

#### Associating the schema to your app

In order for ActyxOS to know that this schema defines the settings for your app, you provide the path to in your app manifest (which also [has a schema](../api/app-manifest-schema.md)):

```yml
manifestVersion: "1.0"
type: web
id: com.example.app1
version: 1.0.3
displayName: App 1
description: "A great first app"
icon: ./build/assets/app-icon.png
dist: ./build/
main: ./build/index.html
settingsSchema: ./settings-schema.json # <------------- Path to your settings schema
```

When you package your app, the Actyx CLI will automatically include the settings schema so that it will be available to ActyxOS when your app is deployed.

#### Accessing settings in your app

The last important part is accessing settings from within your app&mdash;happily knowing that they have been validated against your settings schema. The way this works depends on type of app you have built, or, more precisely, which ActyxOS runtime you are running it in.

**Accessing settings in web apps (WebView Runtime)**

Your app's settings are available in the runtime using an injected global function named `ax.app_config`. To continue with our example, you could access them as follows:

```javascript

function onStartApp() {
  const { timeUnit, backgroundColor } = ax.app_config()
  // Do something with the timeUnit and backgroundColor...
}
```

**Accessing settings in docker apps (Docker Runtime)**

With docker apps, the method is slightly different. In that case, we make your app's settings available as a JSON string in an environment variable called `AX_APP_SETTINGS`. Using the same example but with a docker app written in Python we would access this as follows:

```python
import os
import json

def on_start_app():
  config = json.loads(os.environ['AX_APP_SETTINGS'])
  timeUnit, backgroundColor = config['timeUnit'], config['backgroundColor']
  # Do something with timeUnit and backgroundColor
```

### Configuring apps

Now that we have gone through how you, as an app developer, can define what people can configure using settings, we come to the last part of this page: configuring apps. As shown next, this is completely analogous to [configuring nodes](#configuring-nodes):


```bash
# Create a yml (or JSON) file containing the settings
$ echo "
com.example.app1:
  timeUnit: seconds
  backgroundColor: red" >> app-settings.yml

# Use the Actyx CLI to set the setting on the node at the correct scope
$ ax settings set --local com.example.app1 @app-settings.yml 10.2.3.23
#                         ^                
#                         | Use the app's id as the top-level scope
```

And similarily you can also use mode advanced scopes to selectively set settings within the app's settings tree. Consider for example wanting to change only the background color. You could do so using the following command

```bash
$ ax settings set --local com.example.app1/backgroundColor blue 10.2.3.23
```

:::note Trying to set invalid settings?
ActyxOS validates any settings before applying them. It does so by using the node settings schema as well as the settings schema defined by each app's developer. This ensures only valid settings are ever set.
:::
