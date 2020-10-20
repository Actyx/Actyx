---
title: "Getting Started"
sidebar_label: Getting started
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {DownloadLink} from '../../../src/components/DownloadLink.tsx'

In this chapter, you will run all apps in development mode and interact with them.

To get the sample project, you can either:

Clone the repository from GitHub:

```
git clone https://github.com/Actyx/DemoMachineKit.git
```

Alternatively, you can download the repository as a Zip file, unzip it, and open it in your IDE.

<DownloadLink cta={"Download" } link={"https://github.com/Actyx/DemoMachineKit/archive/master.zip" }/>

### Run the apps

Before you can run the apps, make sure that ActyxOS is running in development mode (check [this guide](https://developer.actyx.com/docs/os/getting-started/installation) for installation on your OS) and has valid settings (check [this guide](https://developer.actyx.com/docs/learn-actyx/tutorial#configure-your-nodes) for node configuration).

:::info Need help?
If you have any issues or just want to give feedback on our tutorial, you are welcome to join our [Discord chat](https://discord.gg/262yJhc), raise an issue in the [GitHub repo](https://github.com/Actyx/DemoMachineKit/issues) or write us an e-mail to developer@actyx.io .
:::

From the root directory, run the following command to start the Dashboard app:

```
npm run ui:dashboard:start
```

After visiting [`localhost:1234`](localhost:1234) you should now see an empty dashboard that only displays "Machines" and "Orders".

For creating orders, lets first start the ERP Simulator App:

```
npm run ui:erp-simulator:start
```

In a different browser window, navigate to [`localhost:1235`](localhost:1235). You should now be able to create an order and associate the following information with it:

- a name
- a planned duration
- a machine

After entering a name and selecting a duration, the `Place order` button will still be greyed out as your system does not know of a machine yet.

In order to let your system know of a new machine, start the Wago Connector App:

```
node:wago-connector:start
```

Now a field with your machine `Wago` should appear, allowing you to click on `Place order`:

[REPLACE IMAGE BELOW]

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/task-system1.png')} />

After placing the order, take another look at the dashboard – you should now see a machine, as well as an order:

[REPLACE IMAGE BELOW]

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/dashboard1.png')} />

As the Wago Connector App running on your development machine is not connected to an actual Wago PLC, your machine will stay in the state `disabled` , and your order will stay in the state `idle`. If your wago-connector app was connected to an actual Wago PLC, it would show its state and your order would change into a different state (such as `running` or `interrupted`), depending on the data it receives from the Wago PLC.

Now that you have a basic understanding of the use case and how the apps work, we will take a closer look at the business logic in the next section.
