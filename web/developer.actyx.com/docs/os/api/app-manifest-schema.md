---
title: App Manifest Schema
---

Each ActyxOS app is described by an **app manifest**.

In order to run them, the runtimes provided by ActyxOS need some information about each app. As a developer you provide this information in the form of a manifest file.

As an example, here is a manifest for a **docker app** for the [Docker Runtime](/os/docs/docker-runtime.html):

```yaml
manifestVersion: "1.0"
type: docker
id: com.example.app1
version: 1.0.3
displayName: App 1
description: "A great first app"
dockerCompose: ./docker-compose.yml
settingsSchema: ./settings-schema.json
```

Here is an example for a **web app** for the [WebView Runtime](/os/docs/webview-runtime.html):

```yaml
manifestVersion: "1.0"
type: web
id: com.example.app1
version: 1.0.3
displayName: App 1
description: "A great first app"
icon: ./build/assets/app-icon.png
dist: ./build/
main: index.html # this is relative to dist
settingsSchema: ./settings-schema.json
```


In order to allow you to validate your manifest files and setup auto-complete in your IDE or text editor, we have defined the schema as a [JSON Schema](https://json-schema.org/). You can download the current schema at:

[https://www.actyx.com/schemas/os/app-manifest.schema.json](/schemas/os/app-manifest.schema.json).

---

> Auto-validation of ActyxOS app manifests in VS Code?
>
> Install the [`vscode-yaml`](https://github.com/redhat-developer/vscode-yaml#associating-a-schema-to-a-glob-pattern-via-yamlschemas) extension and configure it for auto-validating ActyxOS app manifests.
