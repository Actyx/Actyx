---
title: Getting started
---

The **Actyx Command Line Interface (CLI)** is a unified tool to manage your ActyxOS nodes and apps. With the **Actyx CLI**, you can interact with your ActyxOS environment by running commands in your favourite terminal program:

- **Linux or macOS**: Use common shell programs such as [bash](https://www.gnu.org/software/bash/), [zsh](http://www.zsh.org/) or [tcsh](https://www.tcsh.org/) to run commands in linux shells.
- **Windows**: On Windows, run commands with the Windows command prompt or in [PowerShell](https://docs.microsoft.com/en-us/powershell/scripting/overview?view=powershell-7)

## Installation

:::info no help needed?
If you already know how to install command line tools, you can directly go to our [downloads page](https://downloads.actyx.com/) and download the binary file for the Actyx CLI.
:::

This section describes how to install the Actyx CLI. Just choose your operating system and follow the instructions:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

### Requirements
- 64-bit version of Windows

### Installing on Windows

</TabItem>
<TabItem value="unix">

```bash
ax settings set --local com.actyx.os @misc/remote-sample-node-settings.yml <DEVICE_IP>
```

</TabItem>
</Tabs>
