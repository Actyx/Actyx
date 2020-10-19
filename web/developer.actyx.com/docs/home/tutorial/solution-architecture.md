---
title: "Tutorial: A real-world use case"
sidebar_label: Solution architecture
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {ThreeElementRow} from '../../../src/components/ThreeElementRow.tsx'

## Solution architecture

An Actyx solution always consists of several physical devices that each run an instance of ActyxOS. These devices are called ActyxOS nodes. Your Actyx apps will be deployed on the ActyxOS nodes. The example use case in this tutorial entails developing 3 apps:

<ThreeElementRow
    img1={useBaseUrl('images/tutorials/dx1-tutorial/erp-simulator-icon.svg')}
    img2={useBaseUrl('images/tutorials/dx1-tutorial/machine-app-icon.svg')}
    img3={useBaseUrl('images/tutorials/dx1-tutorial/dashboard-app-icon.svg')}
    title1={"ERP Simulator App"}
    title2={"Machine App"}
    title3={"Dashboard App"}
    body1={"Runs on a mobile phone or tablet and lets the user create and manage production orders."}
    body2={"Runs on a machine gateway to start and finish production orders and collect machine data."}
    body3={"Runs on a mobile phone or tablet showing a dashboard displaying relevant production data."}
    showLinks={false}
/>

The below picture shows what the solution would look like in a production-scenario with 3 nodes.

<img src={useBaseUrl('static/images/tutorials/dx1-tutorial/tutorial-setup-02.svg')} />

As you might not have Android mobile phones and a machine gateway available, you will run all apps on only one ActyxOS node in this tutorial: your development machine. The functionality is the same, no matter where your apps are run. In the last section of this tutorial, we will point you to documentation on how to pacakge and deploy these apps to actual devices.

:::info
Add info about running ActyxOS in development mode, and running apps in the browser/node vs. on ActyxOS runtimes
:::
