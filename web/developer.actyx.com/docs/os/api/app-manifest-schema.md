---
title: App Manifest Schema
hide_table_of_contents: true
---

Each ActyxOS app is described by an **app manifest**.

In order to run them, the runtimes provided by ActyxOS need some information about each app. As a developer you provide this information in the form of a manifest file.

As an example, here is a manifest for a **docker app** for the [Docker Runtime](/docs/os/advanced-guides/actyxos-on-docker):

```yaml
manifestVersion: "1.0"
type: docker
id: com.example.app1
version: 1.0.3
displayName: App 1
description: "A great first app"
dockerCompose:
  x86_64: ./docker-compose-amd64.yml
  aarch64: ./docker-compose-arm64v8.yml
settingsSchema: ./settings-schema.json # <---- you could also inline the settings schema
```

Here is an example for a **web app** for the [WebView Runtime](/docs/os/advanced-guides/app-runtimes):

```yaml
manifestVersion: "1.0"
type: web
id: com.example.app1
version: 1.0.3
displayName: App 1
description: "A great first app"
icon: ./build/assets/app-icon.png # Specifying the app icon is optional. If you don't specify an icon for your app, ActyxOS will automatically use a default icon.
dist: ./build/
main: ./index.html
settingsSchema: ./settings-schema.json # <---- you could also inline the settings schema
```

In order to allow you to validate your manifest files and setup auto-complete in your IDE or text editor, we have defined the schema as a [JSON Schema](https://json-schema.org/). You can download the current schema at:

[https://developer.actyx.com/schemas/os/app-manifest.schema.json](/schemas/os/app-manifest.schema.json).

:::tip Use Auto-validation of ActyxOS app manifests in VS Code?
Check out our [guide](/docs/os/advanced-guides/using-vscode-for-schema-validation) on how to setup VS Code for automatic JSON schema validation.
:::
