---
title: Using Visual Studio Code for settings schema validation
hide_table_of_contents: true
---

With the [ActyxOS node settings schema](https://actyx.com/schemas/os/node-settings.schema.json) or any other [JSON Schema](https://json-schema.org/) of one of your apps, you can automatically validate your settings in Visual Studio Code while you write them. This is how you setup the automatic validation:

1. Install the [YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
You can install the extension this directly in Visual Studio Code. Just navigate to Extensions, search for the YAML extension and install it

2. Configure your Visual Studio Code extension

First, go to settings and then search for "schema" and click on "Edit in settings.json" to configure the extension.

In the settings.json file, add the following to validate all files ending in `node.yml` against the ActyxOS node settings schema:

```json
    "yaml.schemas": {
        "https://developer.actyx.com/schemas/os/node-settings.schema.json": "node.yml",
    }
```

You can also use JSON schemas from your local machine and add another JSON schema for your app:

```json
    "yaml.schemas": {
        "./node-settings-schema.json": "node.yml",
        "./sapconnector-settings-schema.json": "sapconnector.yml",
    }
```
