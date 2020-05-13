---
title: Getting started
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The **Actyx Command Line Interface (CLI)** is a unified tool to manage your ActyxOS nodes and apps. With the **Actyx CLI**, you can interact with your ActyxOS environment by running commands in your favourite terminal program:

- **Linux or macOS**: Use common shell programs such as [bash](https://www.gnu.org/software/bash/), [zsh](http://www.zsh.org/) or [tcsh](https://www.tcsh.org/) to run commands in linux shells.
- **Windows**: On Windows, run commands with the Windows command prompt or in [PowerShell](https://docs.microsoft.com/en-us/powershell/scripting/overview?view=powershell-7)

## Installation

:::info no help needed?
If you already know how to install command line tools, you can directly go to our [downloads page](https://downloads.actyx.com/) and download the binary file for the Actyx CLI.
:::

This section describes how to install the Actyx CLI. Just choose your operating system and follow the instructions:

### Requirements

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/macOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

- 64-bit version of Windows
- Admin rights to run the Windows installer

</TabItem>
<TabItem value="unix">

- 64-bit version of macOS

or Linux based on one of the following architectures:

- x64
- armv7hf
- arm

</TabItem>
</Tabs>

### Installing the Actyx CLI

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/macOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

1. Download the Actyx CLI MSI installer for Windows on our [downloads page](https://downloads.actyx.com/). By default, the Actyx CLI installs to `C:\Program Files\Actyx\Actyx CLI X.X.X\`

2. Run the downloaded MSI installer and follow the onscreen instructions

3. To confirm the installation, use the `ax --version` command in PowerShell or a command prompt (open the Start menu and search for PowerShell or cmd to start them). You should see something like this:

```
C:\> ax --version
Actyx CLI 1.0.0-RC1
```

</TabItem>
<TabItem value="unix">

1. Download the Actyx CLI binary file from our [downloads page](https://downloads.actyx.com/)

2. Run the following command to move the Actyx CLI binary file:

```
mv ~/Downloads/ax /usr/local/bin
```

3. Make the Actyx CLI binary file exectuable:

```
chmod +x /usr/local/bin/ax
```

:::caution Additional steps if you use macOS
If you use macOS, you need to additonally allow the Actyx CLI in two steps. First, go to **Settings** and then to **Security & Privacy**. In the **General** tab, you should see a prompt at the bottom that asks you to allow the Actyx CLI. Second, the first time you run an `ax` command, you will be prompted again to allow the Actyx CLI.
:::

4. To confirm the installation, use the `ax --version` command. You should see something like this:

```
$  ax --version
Actyx CLI 1.0.0-RC1
```

</TabItem>
</Tabs>

If you have problems with installing the Actyx CLI, please check our [troubleshooting section](#troubleshooting).

### Uninstalling the Actyx CLI

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/macOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

To uninstall the Actyx CLI, open the Control Panel, go to **Settings** and then to **Apps**. Select the Entry named Actyx CLI, and then choose Uninstall to launch the uninstaller. Confirm that you want to uninstall the Actyx CLI when you're prompted.

</TabItem>
<TabItem value="unix">

Run the following command:

```
rm /usr/local/bin/ax
```

</TabItem>
</Tabs>

## Troubleshooting

If you have any issues or just want to give feedback on our quickstart guide, you are welcome to join our [Discord chat](https://discord.gg/262yJhc) or write us an e-mail to contact@actyx.io .