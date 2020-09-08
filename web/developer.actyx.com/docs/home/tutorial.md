---
title: "Tutorial: Intro to Actyx"
sidebar_label: Tutorial
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

In this tutorial, we are going to build a small chat app on top of the Actyx Platform. **You might be tempted to skip it because you are not building chats in real-life — give it a chance.** The techniques that you will learn in this tutorial are fundamental to building any app on the platform, and mastering them will give you a good understanding of its capabilities.

## Before we start

The tutorial is divided into three sections:

- [Setup for the tutorial](#setup-for-the-tutorial) will give you a starting point to follow the tutorial
- [Overview](#overview) will teach you the fundamentals of Actyx: nodes, events, and fishes
- [Building the chat](#building-the-chat) will teach you the most common techniques in Actyx development

You don’t have to complete all of the sections at once to get value out of this tutorial. Try to get as far as you can — even if it’s just one or two sections.

### Prerequisites

In order to get the most out of this tutorial it is helpful if you are somewhat familiar with HTML, CSS and TypeScript. But since this is a fairly simple app you should be able to follow along even if you are coming from a different programming language.

:::info Typescript in 5 minutes
If you haven't worked in TypeScript before, we can recommend [this guide](https://www.typescriptlang.org/docs/handbook/typescript-in-5-minutes.html) to brush up on the basics.
:::

The best way to experience this tutorial is using multiple devices. In addition to your PC, you will need a second device running Android.

:::warning mDNS and Client Isolation
If you have disabled mDNS in your network, you will have to ensure your devices can initially connect to the internet. If you have enabled Client Isolation, this tutorial will not work.
:::

## Setup for the tutorial

### Setup a Docker device

If you don't have Docker installed on your PC already, you can install it from [here](https://docs.docker.com/get-docker/).

ActyxOS on Docker is publicly available on Docker Hub. To download and run the latest version please execute the following command from your CLI.

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/MacOS', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker run --name actyxos -it --rm -e AX_DEV_MODE=1 -v actyxos_data:/data --privileged -p 4001:4001 -p 4457:4457 -p 4243:4243 -p 4454:4454 actyx/os
```

</TabItem>
<TabItem value="unix">

```
docker run --name actyxos -it --rm -v actyx-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

If you get stuck or want to learn more about ActyxOS on Docker check out [this guide](/os/advanced-guides/actyxos-on-docker.md).

### Setup an Android device

ActyxOS on Android is publicly available from the [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android&hl=en). Just open the Google Play store on your Android device, search for ActyxOS and install it. To start ActyxOS, just open the app like any other.

If you get stuck or want to learn more about ActyxOS on Android check out [this guide](/os/advanced-guides/actyxos-on-android.md)

### Configure your nodes

Now that you have two devices running ActyxOS, note their IP addresses. On Android, you can find the IP address from the ActyxOS System Info tab or directly from your settings. For your local machine, it depends on the operating system that you are running. A quick online search should do the job.

On your local machine now install the ActyxOS Node Manager, which you can download from [downloads.actyx.com](https://downloads.actyx.com). Once installed, use it to connect to each of the ActyxOS nodes using the devices' IP addresses. Then navigate to the _Settings_ tab and paste the following settings for the `com.actyx.os` namespace and click _Save_.

```json
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

This is what it should look like approximately:

![Set node settings using the ActyxOS Node Manager](/images/tutorials/chat-tutorial/set-settings-using-node-manager.png)

If everything has worked, you should see the ActyxOS node running on both devices as shown below:

![Correctly running ActyxOS node in Node Manager](/images/tutorials/chat-tutorial/working-actyxos-node-in-node-manager.png)

### Setup a web app project

In order to be able to run, test and build the chat app you are going to need Node.js and npm, which you can install from [here](https://nodejs.org/en/).

We are now going to setup a simple web app project using [Parcel](https://parceljs.org/). Somewhere on your computer create a directory called `chat`.

In that directory create a file called `package.json` and add the following content:

```json
{
  "name": "decentralized-chat",
  "version": "1.0.0",
  "description": "A decentralized chat",
  "scripts": {
    "build": "tsc && parcel build index.html --public-url .",
    "start": "tsc && parcel index.html"
  },
  "dependencies": {
    "@actyx/pond": "^2.0.1"
  },
  "devDependencies": {
    "@types/node": "^14.0.27",
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

Finally, create a file named `index.ts` with the following:

```ts
console.log('Hello, world!')
```

To test that everything works, open a terminal, navigate to the `chat` directory and run `npm install` and then `npm run start`. This is what you should see in your terminal.

![npm run start](/images/tutorials/chat-tutorial/chat-npm-run-start-post-setup.png)

If you now navigate to [http://localhost:1234](http://localhost:1234) in your browser and open the Developer Tools you should see this:

![Chat in browser](/images/tutorials/chat-tutorial/chat-setup-in-browser.png)

### Help, I’m stuck!

If you get stuck, get help in the [Actyx Developer Chat](https://discord.gg/262yJhc) or e-mail us at developer@actyx.io.

## Overview

Now that you’re set up, let’s get an overview of the Actyx platform!

### What is ActyxOS?

ActyxOS is a multi-node operating system that allows you to build edge native applications running in a swarm of nodes (devices). Specifically you can:

1. Run one ore more apps on each node using the ActyxOS Runtimes
1. Access _always-available_ `localhost` APIs such as the Event Service
1. Count on automatic dissemination and persistence of data in the swarm

![ActyxOS schematic](/images/tutorials/chat-tutorial/actyxos-app-and-communication.png)

ActyxOS enables a completely decentral architecture that allows you to **build apps that always run**. Your apps always run because they run locally (on the edge) and only interact with `localhost` APIs. Currently ActyxOS offers two APIs:

- The **Event Service** API at `http://localhost:4454/api/v1/events` allows you to publish and receive events in the swarm of nodes
- The **Console Service** API at `http://localhost:4457/api/v1/logs` allows you to generate logs for monitoring and debugging

If you want to learn more about ActyxOS, check out our [guides](/os/guides/overview.md) and [advanced guides](/os/advanced-guides/overview.md).

### What is Actyx Pond?

Actyx Pond is an application framework for building apps that run on ActyxOS. It is currently available for the TypeScript programming language. _Support for further languages, inlcuding C#/.NET is planned._ Here is how to works:

1. You implement the business logic of your application by writing so-called _fishes_ and run those in ActyxOS apps
1. Actyx Pond then automatically synchronizes the state of all fishes throughout the swarm of nodes

![Actyx Pond schematic](/images/tutorials/chat-tutorial/actyx-pond-how-it-works.png)

What is interesting about Actyx Pond is that it **allows you to forget completely about how to synchronize state between nodes** in the swarm. This happens, for example, when one of the nodes goes offline for a while. As soon as it comes back up, Actyx Pond automatically reconciles what happened between all the nodes while they were disconnected from each other.

If you want to learn more about Actyx Pond, please start [here](/pond/introduction.md)

:::info Eventual consistency for a partition tolerant system
Formally speaking, Actyx Pond provides eventual consistency for logic implemented on the partition tolerant ActyxOS.
:::

Let's now have a look at how to use ActyxOS and Actyx Pond to build our decentralized chat app.

## Building the chat

To implement and run our chat app we need to do three things:

1. Install ActyxOS on each node (or device). Already done!
1. Implement our chat logic as a fish
1. Package and run our chat app

![Steps for building the chat](/images/tutorials/chat-tutorial/steps-to-complete-chat.png)

### Chat logic

Our chat has a very simple logic. Any participant can send messages and receives messages sent by all other participants. When a participant joins the chat, he should also receive all past messages that were sent when he wasn't part of the chat.

The way to implement this using Actyx Pond is to write a so-called _fish_. A fish is a state-machine. It has a state which it updates when it receives information from other fishes.

Let's start by defining types for the chat fish's state and the events it can receive. Events it receives from other chat fishes are strings (chat messages). The state of the fish will then be an array of strings. In the `index.ts` file, add the following two lines of code:

```ts
type ChatEvent = string
type ChatState = ChatEvent[]
```

When a fish first starts up, it won't have received any chat messages yet. So let's define the initial state as an empty array:

```ts
const INITIAL_STATE: ChatState = []
```

Now comes the actual logic of our chat, namely how to calculate the chat (which we will show to the user), from the events we have received. We do this by writing a so-called `onEvent` function. In this case, we will simply add the chat messages (`ChatEvent`) we have received to our state (`ChatState`):

```ts
function onEvent(state: ChatState, event: ChatEvent) {
    state.push(event);
    return state;
}
```

This is the complete chat logic. Let's now turn this into a fish.

### The chat fish

In Actyx Pond you implement a fish by creating an object with a couple of properties. You must provide the fish with an ID, an initial state, the `onEvent` function and information about where to get the chat messages from, a so-called _event stream tag_.

First, add the following imports to the top of the `index.ts` file:

```ts
import { FishId, Pond, Fish, Tag } from '@actyx/pond'
```

Now that we have done that, we create the tag for our chat messages and then define the fish itself:

```ts
const chatTag = Tag<ChatEvent>('ChatMessage')
const ChatFish: Fish<ChatState, ChatEvent> = ({
    fishId: FishId.of('ax.example.chat', 'MyChatFish', 0),
    initialState: INITIAL_STATE,
    onEvent: onEvent,
    where: chatTag
})
```

### The user interface

Lastly, we need to build a user interface and hook up our fish. Let's implement a very simple user interface showing the chat messages, an input field to type a message and a button to send the message.

Open up the `index.html` file and adjust the contents of the `head` and `body` sections as follows:

```html
<html>
  <head>
    <title>Chat App</title>
    <style>
      body {
        padding: 20px;
      }
      pre {
        height: 300px;
        padding: 10px;
        background-color: #d9d9d9;
        overflow-y: auto;
      }
      button {
        margin-top: 10px;
      }
      button,
      input {
        width: 100%;
        height: 30px;
      }
    </style>
  </head>
  <body>
    <pre id="messages"></pre>
    <input id="message" type="text" />
    <button id="send">send</button>
  </body>
  <script src="./index.js" type="text/javascript"></script>
</html>
```

The last thing we have to do is to hook up the user interface to the fish. We want to

1. Show all chat messages, i.e. fish's state in the `pre` element
1. Send out a chat message event when the user clicks the _Send_ button

In the `index.ts` file, add the following code:

```ts
Pond.default().then(pond => {
    // Select UI elements in the DOM
    const messagesTextArea = document.getElementById('messages')
    const messageInput = <HTMLInputElement>document.getElementById('message')
    const sendButton = document.getElementById('send')

    function clearInputAndSendToStream() {
        // When click on send button get the text written in the input field
        const message = messageInput.value
        messageInput.value = ''
        // Send the message to a stream tagged with our chat tag
        pond.emit(chatTag, message)
    }

    sendButton.addEventListener('click', clearInputAndSendToStream)

    // Observe our chat fish. This means that our callback function will
    // be called anytime the state of the fish changes
    pond.observe(ChatFish, state => {
        // Get the `pre` element and add all chat messages to that element
        messagesTextArea.innerHTML = state.join('\n')
        // Scroll the element to the bottom when it is updated
        messagesTextArea.scrollTop = messagesTextArea.scrollHeight
    });
}).catch(console.log)
```

To test that everything works navigate to the `chat` directory, run `npm run start` and open [http://localhost:1234](http://localhost:1234). You should now see the chat app. If you started ActyxOS on Docker on your local machine, you should now also be able to send messages. However, you still don't have anyone to chat with.

### Package and run the app

In order to run the chat app on your Android device, you need to package and deploy it.

To tell ActyxOS about the app, add an app manifest file called `ax-manifest.yml` to the `chat` directory. Add the following contents:

```yml
manifestVersion: "1.0"
type: web
id: com.actyx.example.chat
version: 1.0.0
displayName: Chat
description: "Peer-to-Peer Chat"
dist: ./dist/
main: ./dist/index.html
settingsSchema: { default: { } }
```

Before packaging the app, run `npm run build` to create a distribution version of the web app (which will be placed in the `dist` directory). If everything works you should see something like this:

![npm run build](/images/tutorials/chat-tutorial/npm-run-build.png)

Now use the ActyxOS Node Manager to package and deploy the app (use the path to the `chat` directory):

![Package app with Node Manager](/images/tutorials/chat-tutorial/node-manager-package-app.png)

![Deploy app with Node Manager](/images/tutorials/chat-tutorial/node-manager-deploy-app.png)

If you open ActyxOS on the Android device, you should now see the chat app. You can click and open the app and should now be able to chat back and forth with your local machine!

![Open chat on Android](/images/tutorials/chat-tutorial/find-open-use-chat-app.png)

You have just built a multi-node application that would traditionally have required a web server and a shared database or pub-sub broker. Here is what you can try out now:

- Do the chat messages get shared between nodes?
- Do I see history if I restart the app?
- What happens if I chat or restart in airplane mode?
- How are messages sorted after I reconnect a device?

We hope that you are now starting to experience the power of edge native applications and the Actyx Platform. If you are keen to dive a bit deeper, check out the following further resources.

## Further resources

- Learn more about [ActyxOS](/os/introduction.md)
- Learn more about [Actyx Pond](/pond/introduction.md)
- Check out the additional libraries and tools such as the:
  - Actyx Pond [VS Code extension](https://marketplace.visualstudio.com/items?itemName=Actyx.actyx-pond) for efficiently writing fishes
  - [React-Pond](https://github.com/actyx-contrib/react-pond) library to quickly integrate with React
  - ActyxOS SDKs for [Rust](./os/sdks/rust.md) and [JS/TS](./os/sdks/js-ts.md)

:::note Join our Discord chat
Feel free to join our [Actyx Developer Chat](https://discord.gg/262yJhc) on Discord. We would love to hear about what you want to build on the Actyx platform.
:::
