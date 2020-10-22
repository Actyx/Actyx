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

Clone the repository from GitHub by running:

```
git clone https://github.com/Actyx/DemoMachineKit.git
```

Alternatively, you can download the repository as a Zip file, unzip it, and open it in your IDE.

<DownloadLink cta={"Download" } link={"https://github.com/Actyx/DemoMachineKit/archive/master.zip" }/>

### Run the apps

Before you can run the apps, make sure that ActyxOS is running in development mode (check [this guide](https://developer.actyx.com/docs/os/getting-started/installation) for installation on your OS) and has valid settings (check [this guide](https://developer.actyx.com/docs/learn-actyx/tutorial#configure-your-nodes) for node configuration).

:::info Need help?
If you run into problems or want to give feedback, you are welcome to join our [Discord chat](https://discord.gg/262yJhc), raise an issue in the [GitHub repo](https://github.com/Actyx/DemoMachineKit/issues) or write us an e-mail to developer@actyx.io.
:::

Before you start any apps, run `npm install` from the root directory.

To start the Dashboard app, run:

```
npm run ui:dashboard:start
```

After visiting `localhost:1234` you should see an empty dashboard that only displays "Machines" and "Orders".

For creating orders, lets first start the ERP Simulator App:

```
npm run ui:erp-simulator:start
```

In a different browser window, navigate to `localhost:1235`. You should now be able to create an order and associate the following information with it:

- a name
- a planned duration
- a machine

After entering a name and selecting a duration, the `Place order` button will still be greyed out as your system does not know of a machine yet.

In order to let your system know of a new machine, start the Wago Connector App:

```
npm run node:wago-connector:start
```

Now a field with your machine `Wago` should appear, allowing you to click on `Place order`:

<img src={useBaseUrl('images/tutorials/dx1-tutorial/erpsimulator.png')} />

After placing the order, take another look at the dashboard – you should now see a machine, as well as an order:

<img src={useBaseUrl('images/tutorials/dx1-tutorial/dashboard1.png')} />

As the Wago Connector App running on your development machine is not connected to an actual Wago PLC, your machine will stay in the state `disabled`, and your order will stay in the state `idle`. If your Wago Connector App was connected to an actual Wago PLC, it would show its state and your order would change into a different state (such as `running` or `interrupted`), depending on the data it receives from the Wago PLC.

Now that you have a basic understanding of the use case and how the apps work, we will take a closer look at the business logic in the next section.
