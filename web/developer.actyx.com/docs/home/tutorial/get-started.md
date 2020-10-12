---
title: "Tutorial: A real-world use case"
sidebar_label: Getting started
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {DownloadLink} from '../../../src/components/DownloadLink.tsx'

## Getting started

In this chapter, we will first have a look at the structure of the project and then run 2 of the 3 apps in development mode.

To get the sample project, you can either:

Clone the repository from GitHub:

```
git clone https://github.com/Actyx/DemoMachineKit.git
```


Alternatively, you can download the repository as a Zip file, unzip it, and open it in Android Studio.

<DownloadLink cta={"Download" } link={"https://github.com/Actyx/DemoMachineKit/archive/master.zip" }/>

### Open the project in your IDE

Inspect the project. It consists of a few configuration files, as well as  3 apps in the `src` folder: `dashboard`, `task-system`, `wago-connector`. The `fish` folder contains all relevant fish used in the apps.

**[REPLACE IMAGE BELOW]**

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/project.png')} />

We will take a closer look at the structure in the next chapter.

### Run the apps

Before you can run the apps, make sure that ActyxOS is running in development mode (check [this guide](https://developer.actyx.com/docs/os/getting-started/installation) for installation on your OS) and has valid settings (check [this guide](https://developer.actyx.com/docs/learn-actyx/tutorial#configure-your-nodes) for node configuration).

From the root directory, run the following commands to start the dashboard and the dashboard app:

```
npm run ui:dashboard:start
```

After visiting [`localhost:1234`](localhost:1234) you should now see an empty dashboard that only displays "Machines" and "Orders".

For creating orders, lets first start the order creation app:

```
npm run ui:task-system:start
```

As port 1234 is already taken by the dashboard, it will be assigned to a random port – just check the output in your terminal and, in a different browser window, navigate to `localhost:<PORT>`. You should now be able to create a order and associate the following information with it:

- a name
- a planned duration
- a machine

After entering a name and selecting a duration, the `Place task" button will still be greyed out as your system does not know of a machine yet.

In order to let your system know of a new machine, start the wago-connector app:

```
node:wago-connector:start
```

Now a field with your machine `Wago` should appear, allowing you to click on `Place task`:

**[REPLACE IMAGE BELOW]**

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/task-system1.png')} />

After placing the ask, take another look at the dashboard – you should now see a machine, as well as a order:

**[REPLACE IMAGE BELOW]**

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/dashboard1.png')} />

As the wago-connector app running on your development machine is not connected to an actual Wago PLC, your machine will stay in the state `disabled` , and your order will stay in the state `idle`. If your wago-connector app was connected to an actual Wago PLC, it would show its state and your order would change into a different state (such as `running` or `interrupted`), depending on the data it receives from the Wago PLC.

Now that you have a basic understanding of the use case and how the apps work, we will take a closer look at the business logic in the next section.