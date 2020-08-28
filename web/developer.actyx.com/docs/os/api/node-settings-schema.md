---
title: Node Settings Schema
---

The format of ActyxOS node settings is defined in the **Node Setting Schema**.

ActyxOS needs a number of settings to be defined in order to work, and provides optional settings that you can use to configure your nodes' behavior. Which settings are available and which values they may have is defined in the so-called ActyxOS **Node Setting Schema**. The most recent version thereof is available for download at:

[https://developer.actyx.com/schemas/os/node-settings.schema.json](/schemas/os/node-settings.schema.json)

> Auto-validation of ActyxOS app manifests in VS Code?
>
> Install the [`vscode-yaml`](https://github.com/redhat-developer/vscode-yaml#associating-a-schema-to-a-glob-pattern-via-yamlschemas) extension and configure it for auto-validating ActyxOS app manifests.

Here is an example of schema-compliant node settings:

```yaml
general:
  displayName: My Node
  swarmKey: 4904199ec5e74cc5871cad1ddad4b9e636c9dfcc55269d954dd4048e336b5433
  bootstrapNodes:
    - /ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH
  logLevels:
    OS: WARN
    Apps: INFO
licensing:
  os: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  apps:
    com.example.app1: development
    com.example.app2: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
services:
  eventService:
    topic: My Topic
    readOnly: false
```

If this were stored in a file named `settings.yml`, you could now use the [Actyx CLI](/os/docs/actyx-cli.html) to set these setting as follows:

```bash
ax settings set --local com.actyx.os @settings.yml 10.2.3.23
```

:::caution ActyxOS on Android only supports MultiAddrs for bootstrapNode
ActyxOS on Android is currently not able to resolve DNS names inside MultiAddrs and thus only supports ip4 or ip6 MultiAddrs. For example, if you want to connect to the public ActyxOS Bootstrap Node, you have to set the value

- /ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH

instead of

- /dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH

ActyxOS on Docker supports both formats. We are currently working on a fix for this. Check out our [blog](https://www.actyx.com/news/) or [release notes section](/docs/os/release-notes.md) for information on our new releases.
:::
