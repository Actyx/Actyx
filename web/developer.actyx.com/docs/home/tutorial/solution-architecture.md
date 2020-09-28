---
title: "Tutorial: A real-world use case"
sidebar_label: Solution architecture
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';

## Solution architecture

An Actyx solution always consists of several physical devices that each run an instance of ActyxOS. These devices are called ActyxOS nodes. Your Actyx apps will be deployed on the ActyxOS nodes. The example use case in this tutorial entails developing 3 apps:

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/apps.png')} />

The above picture shows how the solution would look like in a production-scenario with 3 nodes: 
- An mobile phone or tablet that you use to create orders
- A machine gateway that starts and finishes production orders and collects machine data
- A mobile phone or tablet that displays a dashboard

As you might not have Android mobile phones or tablets available, we will first run the Order Import app as well as the Dashboard app on only one ActyxOS node: your PC. The functionality is the same, no matter where your apps are run.

:::info
In the last (optional) chapter of this tutorial, you will then package and deploy these apps to actual Android devices.
:::