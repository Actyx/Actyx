---
title: "Tutorial: Intro to Actyx"
sidebar_label: Tutorial
---

This tutorial that doesn't assume any existing knowledge of the Actyx Platform.

## Before we start

We are going to build a small chat app during this tutorial. **You might be tempted to skip it because you are not building chats in real-life — give it a chance.** The techniques that you will learn in this tutorial are fundamental to building any app on the Actyx platform, and mastering them will give you a deep understanding of the platform.

The tutorial is divided into several sections:

- Setup for the Tutorial will give you a starting point to follow the tutorial.
- Overview will teach you the fundamentals of Actyx: nodes, fishes, and events.
- Completing the chat will teach you the most common techniques in Actyx development.
- Adding time travel will give you a deeper insight into the unique strengths of Actyx.

You don’t have to complete all of the sections at once to get the value out of this tutorial. Try to get as far as you can — even if it’s one or two sections.

### What are we building?

In this tutorial we will show you how to build a decentralized chat app. The result will be an app that you can run on your phone (or your computer) and that will look something like this:

![hello-chat-screenshot](/images/home/hello-chat.jpg)

### Prerequisites

We’ll assume that you have some familiarity with HTML, CSS and Typescript, but you should be able to follow along even if you’re coming from a different programming language. We’ll also assume that you’re familiar with programming concepts like functions, objects and arrays.


:::info Typescript in 5 minutes
If you need to review Typescript, we recommend reading [this guide](https://www.typescriptlang.org/docs/handbook/typescript-in-5-minutes.html). 
:::

The best way to experience this tutorial — and decentralized computing — is using multiple devices. If you don't have two Android devices at hand, you can also use your laptop and one Android device. You will have to make sure that both of these devices are connected in the same Local Area Network.

:::warning mDNS and Client Isolation
If you have disabled mDNS in your network, you will have to ensure your devices can initially connect to the internet. If you have enable Client Isolation, this tutorial will not work.
:::

## Setup for the tutorial

### Setup two devices

On two different Android devices install ActyxOS via the PlayStore ([link](https://play.google.com/store/apps/details?id=com.actyx.os.android&hl=en_US)).

![ActyxOS in the PlayStore](/images/home/playstore-install-actyxos.png)

Once installed, start the ActyxOS app, access the _System Info_ screen and note your devices' IP addresses (in this example it is `192.168.1.141`)

![ActyxOS System Info](/images/home/actyxos-get-ip-address.png)

On your local machine install the ActyxOS Node Manager, which you can download from [downloads.actyx.com](https://downloads.actyx.com). Once installed use it to connect to each one of the ActyxOS nodes using the devices' IP addresses. Then navigate to the _Settings_ tab and paste the following settings for the `com.actyx.os` namespace and click _Save_.

```yaml
{
   "general": {
      "bootstrapNodes": [
         "/ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH"
      ],
      "displayName": "Remote Sample Node",
      "swarmKey": "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK"
   },
   "licensing": {
      "os": "development",
      "apps": {}
   },
   "services": {
      "consoleService": {},
      "eventService": {
         "readOnly": false,
         "topic": "SampleTopic"
      },
      "dockerRuntime": {},
      "webViewRuntime": {}
   }
}
```

This is what is should look like approximately:

![Set node settings using the ActyxOS Node Manager](/images/home/set-settings-using-node-manager.png)

If everything has worked, you should see the ActyxOS node running on both devices as shown below:

![Correctly running ActyxOS node in Node Manager](/images/home/working-actyxos-node-in-node-manager.png)

### Setup a web app project

We are now going to setup a simple web app project using [Parcel](https://parceljs.org/). Somewhere on your computer create a directory called `chat`.

In that directory create a file called `package.json` and add the following content:

```json
{
  "name": "decentralized-chat",
  "version": "1.0.0",
  "description": "a decentralized chat",
  "main": "index.ts",
  "scripts": {
    "build": "tsc && parcel build index.html --public-url .",
    "start": "tsc && parcel index.html",
    "test": "echo \"error: no test specified\" && exit 1"
  },
  "author": "",
  "license": "isc",
  "dependencies": {
    "@actyx/pond": "^2.0.1"
  },
  "devDependencies": {
    "@types/node": "^13.9.0",
    "parcel-bundler": "^1.12.4",
    "typescript": "^3.9.7"
  }
}
```

Create another file called `tsconfig.json` with the following content:

```json
{
    "compilerOptions": {
        "esModuleInterop": true,
        "sourceMap": true
    }
}
```

Now create a file called `index.html` and add the following:

```html
<html>
    <head>
    </head>

    <body>
    <p>A chat is coming soon!</p>
    </body>
    <script src="./index.js"></script>
</html>
```

Finally create a file named `index.ts` with the following:

```ts
console.log('Hello, world!')
```

To test that everything works, open a terminal, navigate to the `chat` directory and run `npm install` and then `npm run start`. This is what you should see in your terminal.

![](/images/home/chat-npm-run-start-post-setup.png)

If you now navigate to (http://localhost:1234) in your browser and open the Developer Tools you should see this:

![](/images/home/chat-setup-in-browser.png)



That's it 😀. You are now ready to build the chat!


## Overview

## Completing the chat

## Where to go now



