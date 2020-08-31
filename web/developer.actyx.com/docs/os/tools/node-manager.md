---
title: ActyxOS Node Manager
---

<!-- Add as react component to be able to handle the width (otherwise it goes full width) -->
<img src="/images/os/node-manager-icon.png" style={{maxWidth: "150px", marginBottom: "1rem" }} />

The ActyxOS Node Manager provides an easy-to-use GUI to interact with ActyxOS nodes. It provides all functionalities you need for monitoring and configuring your nodes and packaging and deploying apps. The main elements of the applications are the status bar and 5 tabs, which will be explained in more detail in the following. 

### Status Bar
The status bar lets you connect to your nodes by entering the IP address of the node you want to connect to into the text field and hitting the _Connect_ button. In addition, the bar displays the current connection status of your node at all times on the left.

### Status Tab

The _Status Tab_ displays general information about the node you are currently connected to. It displays the following information:

- Connection state: `unreachable` or `reachable`
- Node ID: `<IP address>` or `localhost`
- Display name: The display name you defined in the node settings
- State: `Stopped` or `Running`
- Settings: `Invalid` or `Valid`
- License: `Invalid` or `Valid`
- Apps deployed: Number of apps that are currently deployed to the node
- Apps running: Number of apps that are currently running on the node
- Started: Date and time when ActyxOS was started on the node
- Version: Version number of ActyxOS running on the node

:::info Node and app lifecycle
For more information on the node's general state or settings and license states, please refer to the [node and app lifecycle documentation](http://localhost:3000/docs/os/advanced-guides/node-and-app-lifecycle/).
:::

Additionally, the _Status Tab_ also displays the logs emitted by the ActyxOS node.


<img src="/images/os/node-manager-status.png" style={{maxWidth: "550px", marginBottom: "1rem" }} />

### Apps Tab

The _Apps Tab_ gives an overview of all applications that are installed on the node in one glance. The table displays the following information:

- App ID: The ID you specify in the apps `manifest.yml`. This is also the settings scope of this application.
- Version: Version number of the application
- Enabled: Status of the application (_Disabled_ or _Enabled_). For more info please refer to the [application lifecycle](https://developer.actyx.com/docs/os/advanced-guides/node-and-app-lifecycle/).
- State: `Stopped` or `Running`
- Settings: `Invalid` or `Valid`
- License: `Invalid` or `Valid`
- Started: Date and time when the application was started
- Actions: Options to start, stop and undeploy the application

:::info Node and app lifecycle
For more information on the node's general state or settings and license states, please refer to the [node and app lifecycle documentation](http://localhost:3000/docs/os/advanced-guides/node-and-app-lifecycle/).
:::

Moreover, the _Apps Tab_ offers the capability to validate and package an application and deploy applications to nodes. In case you want to validate or package an application, please enter the path to the app directory into the text field. If you want to deploy an application, please enter the path to the packaged tar.gz file into the text field. 

<img src="/images/os/node-manager-apps.png" style={{maxWidth: "550px", marginBottom: "1rem" }} />

### Settings Tab

The _Settings Tab_ displays all [settings scopes](https://developer.actyx.com/docs/os/advanced-guides/node-and-app-settings/#configuring-nodes) that are deployed to the node and their respective settings in an interactive code editor. The scope of the node is `com.actyx.os` and the scope of the apps are their respective app ID. 

You can simply edit the JSON file in the editor to change the settings for your node or for an app. Every time you edit the settings, your changes will be validated against the JSON schema and can only be saved when settings comply with the schema. You can view the settings schema by ticking the checkbox in the bottom right corner. 

<img src="/images/os/node-manager-settings.png" style={{maxWidth: "550px", marginBottom: "1rem" }} />


### Tools Tab
The _Tools Tab_ lets you generate a new swarm key and lets you copy it to the clipboard. A [swarm](https://developer.actyx.com/docs/os/guides/swarms/#whats-a-swarm) is defined by a single property, the so-called swarm key. In order to participate in a swarm, a node must have the secret swarm key. The swarm key is a setting that must be set for a node to function correctly. 

### About Tab
The _About Tab_ displays the Actyx CLI version that the node manager is based on. Additionally you can see the Software License Agreement and links to our support channels. 


## Installation

Please download the application [here](https://downloads.actyx.com/). Supported platforms are Windows, Linux and macOS.

:::info Installing on macOS
Upon installing the ActyxOS Node Manager on macOS, you will need to make a change in your security settings because Actyx is not yet listed as an Apple developer.
:::

1. Click _OK_ on the initial prompt after opening Node Manager for the first time.
<img src="/images/os/installation-01.png" style={{marginBottom: "1rem" }} />

2. Go to the **Security & Privacy** section in your System Preferences application and click on _Open Anyway_.
<img src="/images/os/installation-02.png" style={{marginBottom: "1rem" }} />

3. Finally, open Node Manager again and click _Open_ when prompted with another dialogue.
<img src="/images/os/installation-03.png" style={{marginBottom: "1rem" }} />
