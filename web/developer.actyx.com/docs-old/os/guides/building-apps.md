---
title: Building Apps
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

You encapsulate your business logic on ActyxOS as apps. An app is composed of, at least,

- a unique id (e.g. `com.example.app`),
- a display name (e.g. _Example App_),
- a settings schema (which can also be empty); and,
- the logic itself (e.g. a docker image or a web app)

Currently you can build two types of apps for ActyxOS: _web apps_ and _docker apps_. Let's run through an example.

## App manifest

Create a new directory called `my-app` on your computer and create a file called `ax-manifest.yml` in that directory with the following content:

```yml
manifestVersion: "1.0"
id: com.example.myapp
version: 1.0.0
displayName: Example App
description: "An example app"
settingsSchema: ./settings-schema.json # You can also inline the JSON object that defines your settings schema
```

This content of the manifest file is what you will need irrespective of what kind of app you are building. The directory should now look as follows:

```text
my-app/
|--- ax-manifest.yml
```

## App type

In order to be valid, the manifest file needs an additional property called `type`, the value of which can be either `web` or `docker`. Depending on which type of app you are building you must then add additional properties to the manifest as follows:

<Tabs
  defaultValue="web"
  values={[
    { label: 'web', value: 'web', },
    { label: 'docker', value: 'docker', },
  ]
}>
<TabItem value="web">

```yml
# These properties apply to all types of apps
manifestVersion: "1.0"
id: com.example.myapp
version: 1.0.0
displayName: Example App
description: "An example app"
settingsSchema: ./settings-schema.json # You can also inline the JSON object that defines your settings schema

# Here we define the app type
type: web

# These three properties are specific to apps of type `web`
icon: ./icon.png # Specifying the app icon is optional. If you don't specify an icon for your app, ActyxOS will automatically use a default icon.
dist: ./
main: index.html
```

</TabItem>
<TabItem value="docker">

```yml
# These properties apply to all types of apps
manifestVersion: "1.0"
id: com.example.myapp
version: 1.0.0
displayName: Example App
description: An example app
settingsSchema: ./settings-schema.json # You can also inline the JSON object that defines your settings schema

# Here we define the app type
type: docker

# This one property is specific to apps of type `docker`
dockerCompose:
  x86_64: ./docker-compose-amd64.yml
  aarch64: ./docker-compose-arm64v8.yml
```

</TabItem>
</Tabs>

:::info App Manifest Schema
For more information and a JSON schema of the app manifest file, check out the [App Manifest Schema API reference](../api/app-manifest-schema.md).
:::

## App settings

You must provide a settings schema for your app. This will allow users who want to run your app to safely provide it with settings.

An example settings schema (`settings-schema.json` above), could be:

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

Given this settings type, ActyxOS would know that users can set the language, and that if none has been set, ActyxOS should use the provided default language (`english`).

:::note
For more information about app settings please the advanced guide about [Node and App Settings](../advanced-guides/node-and-app-settings.md).
:::

Your directory structure should now look as follows:

```text
my-app/
|--- ax-manifest.yml
|--- settings.schema.json
```

:::info You have no settings?
If your app is not configurable at all, you can also tell ActyxOS that.
The shortest form just provides an empty object as default, which can also be declared directly within the manifest:

```yaml
settingsSchema: { "default": {} }
```

More details can be found [in this section](../advanced-guides/node-and-app-settings.md#deploying-an-app-without-settings).
:::

## App logic

The final step before packaging and deploying the app is implementing the app logic. Let's run through a simple example for both _web apps_ and _docker apps_.

### Web apps

As a simple example of a web app, create a file called `index.html` and add the following content to it:

```html
<!DOCTYPE html>
<html>
    <body>
    <h1>My App</h1>
    <p>Click the button!</p>
    <button onclick="myFunction()">Click me</button>
    <p id="clicked">Clicks: </p>
    <script>
        function myFunction() {
            document.getElementById("clicked").appendChild(document.createTextNode("click, "));
        }
    </script>
    </body>
</html>
```

This obviously does not use any ActyxOS features, at this point we are only interested in the capability of deploying and running this app on ActyxOS.
We will take a look at more interesting logic in the [event streams](event-streams.md) guide.

:::tip App icon is optional
If you don't have an icon for your app, just omit the `icon` property in your ax-manifest.yml file. ActyxOS will automatically add a default icon.
:::

Your directory should now look as follows:

```text
my-app/
|--- ax-manifest.yml
|--- settings.schema.json
|--- icon.png
|--- index.html
```

### Docker apps

The logic of docker apps is implemented in docker images you create. To show you how this works, we will create a simple Docker image and Docker compose file.

Create a file named `Dockerfile` in your directoy and add the following content to it:

```Dockerfile
FROM alpine
CMD while sleep 1; do date +%T; done
```

This obviously does not use any ActyxOS features, at this point we are only interested in the capability of deploying and running this app on ActyxOS.
We will take a look at more interesting logic in the [event streams](event-streams.md) guide.

Now build the docker image, tagging it (naming it) `myapp` (make sure you are in the `my-app` directory):

```text
docker build --tag myapp .
```

Now create a file called `docker-compose.yml` and add the following content:

```yaml
version: '2.0'
services:
  myapp:
    image: myapp
```

You have now created a docker image on your machine and a docker compose file explaining how to run your app. Your directory should now look as follows:

```text
my-app/
|--- ax-manifest.yml
|--- settings.schema.json
|--- Dockerfile
|--- docker-compose.yml
```

## Packaging your app

In order to run your web or docker app on ActyxOS, you need to package it using the [Actyx CLI](../../cli/getting-started). Run the following command from within the `my-app` directory:

```text
ax apps package .
```

Within your directory you should now find a `.tar.gz` file containing your packaged app. You are now ready to [run your app](running-apps.md).
