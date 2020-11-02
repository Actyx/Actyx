---
title: Node Settings Schema
hide_table_of_contents: true
---

The format of ActyxOS node settings is defined in the **Node Setting Schema**.

ActyxOS needs a number of settings to be defined in order to work, and provides optional settings that you can use to configure your nodes' behavior. Which settings are available and which values they may have is defined in the so-called ActyxOS **Node Setting Schema**. The most recent version thereof is available for download at:

[https://developer.actyx.com/schemas/os/node-settings.schema.json](/schemas/os/node-settings.schema.json)

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

If this were stored in a file named `settings.yml`, you could now use the [Actyx CLI](/docs/cli/getting-started) to set these setting as follows:

```text
ax settings set --local com.actyx.os @settings.yml 10.2.3.23
```
