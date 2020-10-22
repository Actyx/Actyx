---
title: "Solution architecture"
sidebar_label: Solution architecture
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {ThreeElementRow} from '../../../src/components/ThreeElementRow.tsx'

An Actyx solution always consists of several physical devices that each run an instance of ActyxOS. These devices are called ActyxOS nodes. Your Actyx apps will be deployed on the ActyxOS nodes. The example use case in this tutorial entails developing 3 apps:

<ThreeElementRow
    img1={useBaseUrl('images/tutorials/dx1-tutorial/erp-simulator-icon.svg')}
    img2={useBaseUrl('images/tutorials/dx1-tutorial/machine-app-icon.svg')}
    img3={useBaseUrl('images/tutorials/dx1-tutorial/dashboard-app-icon.svg')}
    title1={"ERP Simulator App"}
    title2={"Wago Connector App"}
    title3={"Dashboard App"}
    body1={"Runs on a mobile device and lets the user create and manage production orders."}
    body2={"Runs on a machine gateway to start and finish production orders and collect machine data."}
    body3={"Runs on a mobile device showing a dashboard displaying relevant production data."}
    showLinks={false}
/>

The below picture shows what the solution would look like in a production-scenario with three nodes.

<img src={useBaseUrl('images/tutorials/dx1-tutorial/tutorial-setup-02.svg')} />

Don't worry in case you do not have Android phones, a machine gateway and a Wago PLC available. In this tutorial you will run all apps on only one ActyxOS node: your development machine. The functionality is the same, no matter where your apps run. In the last section of this tutorial, we will point you to documentation on how to package and deploy these apps to actual devices.

:::info Running ActyxOS in development mode
During development, you usually run ActyxOS in development mode. This means that all ActyxOS APIs are exposed on your development machine. Instead of packaging and deploying your app to an ActyxOS runtime, you can then just run it on your PC.
:::
