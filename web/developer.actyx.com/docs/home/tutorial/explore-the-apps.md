---
title: "Explore the apps"
sidebar_label: Explore the apps
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';

In this section, you will explore the source code behind the apps. The first part will briefly explain the overall project structure, and the second and third section will focus on the fishes used by the apps and the apps themselves.

:::info Need help?
If you have any issues or just want to give feedback on our tutorial, you are welcome to join our [Discord chat](https://discord.gg/262yJhc), raise an issue in the [GitHub repo](https://github.com/Actyx/DemoMachineKit/issues) or write us an e-mail to developer@actyx.io .
:::

## Project structure

The project consists of a few configuration files, as well as  3 apps in the `src` folder: `dashboard`, `erp-simulator`, `wago-connector`. The `fish` folder contains all relevant fishes used in the apps.

[REPLACE IMAGE BELOW]

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/project.png')} />

## Fishes

As discussed in more detail in [add link], a fish is the main programming unit in the Actyx Pond framework. It always represents one entity of your business logic and should always have only a single responsibility – which means in this case, you need fishes to keep track of your machines, as well as your orders.

[replace image]

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/fishdirectory.png')} />

The `machineFish.ts` and `taskFish.ts` code is structured into the same sections:

**1. Definition of state types**

**2. Definition of event types**

**3. Definition of tag types**

**4. Definition of fishes**

For this use case, two kinds of fishes are defined for machines as well as orders. Each of the fishes serves a different purpose:

- implement state and logic of a single entity: A fish responsible for a single machine or order
- track and access many instances of an entity: A fish responsible for tracking all machines or all orders

### Machine fish

The following illustration should make it easier to understand `machineFish.ts`. It represents the state- and event types of the machine fish responsible for the state of a single machine:

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/machinefish.png')} />

### Order fish

The following illustration should make it easier to understand `orderFish.ts`. It represents the state- and event types of the order fish responsible for the state of a single order:

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/orderfish.png')} />

As you can see in `orderfish.ts` , `DefinedState` can be, depending on the event, either `idle`, `active`, or `done`.

## The apps