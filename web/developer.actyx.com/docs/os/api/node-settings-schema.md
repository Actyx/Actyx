---
title: Node Settings Schema
---

The format of ActyxOS node settings is defined in the **Node Setting Schema**.

ActyxOS needs a number of settings to be defined in order to work, and provides optional settings that you can use to configure your nodes' behavior. Which settings are available and which values they may have is defined in the so-called ActyxOS **Node Setting Schema**. The most recent version thereof is available for download at:

[https://www.actyx.com/schemas/os/node-settings.schema.json](/schemas/os/node-settings.schema.json).

> Auto-validation of ActyxOS app manifests in VS Code?
>
> Install the [`vscode-yaml`](https://github.com/redhat-developer/vscode-yaml#associating-a-schema-to-a-glob-pattern-via-yamlschemas) extension and configure it for auto-validating ActyxOS app manifests.

Here is an example of schema-compliant node settings:

```yaml
General:
  DisplayName: My Node
  SwarmKey: 4904199ec5e74cc5871cad1ddad4b9e636c9dfcc55269d954dd4048e336b5433
  BootstrapNodes:
    - /ip4/10.2.3.10/tcp/9090
    - /ip4/10.2.3.11/tcp/9090
  LogLevels:
    OS: WARN
    Apps: INFO
Licensing:
  OS: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  Apps:
    com.example.app1: development
    com.example.app2: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
Services:
  EventService:
    Topic: My Topic
    ReadOnly: false
```

If this were stored in a file named `settings.yml`, you could now use the [Actyx CLI](/os/docs/actyx-cli.html) to set these setting as follows:

```bash
$ ax settings set --local ax.os @settings.yml 10.2.3.23
```