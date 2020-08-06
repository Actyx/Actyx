---
title: "Tutorial: Intro to Actyx"
sidebar_label: Tutorial
---

This tutorial that doesn't assume any existing knowledge of the Actyx Platform.

## Before we start

We are going to build a small chat app during this tutorial. **You might be tempted to skip it because you are not building chats in real-life — give it a chance.** The techniques that you will learn in this tutorial are fundamental to building any app on the Actyx platform, and mastering them will give you a deep understanding of the platform.

The tutorial is divided into several sections:

- [Setup for the Tutorial](#setup-for-the-tutorial) will give you a starting point to follow the tutorial.
- [Overview](#overview) will teach you the fundamentals of Actyx: nodes, events, and fishes.
- [Building the chat](##building-the-chat) will teach you the most common techniques in Actyx development.

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

### Help, I’m Stuck!

If you get stuck, get help in the [Actyx Developer Chat](https://discord.gg/262yJhc) or e-mail us at developer@actyx.io. If you don’t receive an answer, or if you remain stuck, please [file an issue](https://github.com/actyx/quickstart), and we’ll help you out.

## Overview

Now that you’re set up, let’s get an overview of the Actyx platform!

### What is the ActyxOS?

ActyxOS is a multi-node operating system that allows you to build distributed applications running in a swarm of nodes (devices). Specifically you can

1. run one ore more apps on each node using the ActyxOS Runtimes
1. access _always-available_ `localhost` APIs such as the Event Service
1. count on automatic dissemination and persistence of data in the swarm

![](/images/tutorial/actyxos-app-and-communication.png)

ActyxOS enables a completely decentral architecture that allows you to **build apps that always run**. Your apps always run because they run locally (on the edge) and only interact with `localhost` APIs. Currently ActyxOS offers two APIs:
- the **Event Service** API at `http://localhost:4454/api/v1/events` allows you to publish and receive events in the swarm of nodes
- the **Console Service** API at `http://localhost:4457/api/v1/logs` allows you to generate logs for monitoring and debugging.


### What is Actyx Pond?

Actyx Pond is an application framework for building apps to run on ActyxOS. It is currently available for the [Typescript](https://www.typescriptlang.org/) programming language. _Support for further languages, inlcuding C#/.NET is planned._ Here is how to works:

1. You implement the business logic of your application by writing so-called _fishes_ and run those in ActyxOS apps
1. Actyx Pond then automatically synchronizes the state of all fishes throughout the swarm of nodes

![](/images/tutorial/actyx-pond-how-it-works.png)

What is interesting about the Actyx Pond is that is **allows you to forget completely about how to synchronize state between nodes** in the swarm. This happens, for example, when one of the nodes goes offline for a while. As soon as it comes back up, the Actyx Pond automatically reconciles what happened between all the nodes while they were disconnected from each other.


:::info Eventual consistency for a partition tolerant system
Formally speaking, Actyx Pond provides eventual consistency for logic implemented on the partition tolerant ActyxOS.
:::

Let's have a look at how to use ActyxOS and Actyx Pond to build a decentralized chat.

## Building the chat

To implement and run our chat app we need to do three things:

1. Install ActyxOS on each node (or device). Already done!
1. Implement our chat logic as a fish
1. Package and run our chat app

![](/images/tutorial/steps-to-complete-chat.png)

### Chat logic

Our chat has a very simple logic. Any participant can send messages and receives all messages sent by all other participants. When a participant joins the chat, he should also receive all past messages that were sent when he wasn't part of the chat.

The way to implement this using Actyx Pond is to write a so-called _fish_. A fish is basically a state-machine. It has a state which it updates when it receives information from other fishes.

Let's start by defining types for the chat fish's state and the event it can receive. The state of the fish will be a list of strings (chat messages). Events it receives from other chat fishes are strings. In the `index.ts` file, add the following two lines of code:

```ts
type ChatEvent = string
type ChatState = ChatEvent[]
```

When a fish first starts up, it won't have received any chat messages yet. So let's define the initial state as an empty array:

```ts
const INITIAL_STATE: ChatState = []
```

Now comes the actual logic of our chat, namely how to calculate the chat (which we will show to the user), from events we have received. We do this by writing a so-called `onEvent` function. In this case, we will simply add the chat messages (`ChatEvent`) we have received to our state (`ChatState`):

```ts
function onEvent(state: ChatState, event: ChatEvent) {
    state.push(event);
    return state;
}
```

This is the complete chat logic. Let's now turn this into a fish.

### The chat fish

In Actyx Pond you implement a fish by creating an object with a couple of properties. You must provide the fish with an ID, an initial state, the `onEvent` function and information about where to get the chat messages from, a so-called _event stream tag_.

First add the following imports to the top of the `index.ts` file:

```ts
import { FishId, Pond, Fish, Tag } from '@actyx/pond'
```

Now that we had done that, we create the tag for our chat messages and then define the fish itself:

```ts
const chatTag = Tag<ChatEvent>('ChatMessage')

const ChatFish: Fish<ChatState, ChatEvent> = ({
    fishId: FishId.of('ax.example.chat', 'MyChatFish', 0),
    initialState: [],
    onEvent: onEvent,
    where: chatTag
});
```

### The user interface

Lastly, we need to build a user interface and hook up our fish. Let's implement a very simple user interface showing the chat messages, an input field to type a message and a button to send the message.

Open up the `index.html` file and adjust the contents of the `head` and `body` sections as follows:

```html
<html>
    <head>
        <style>
            pre {
                width: 90%;
                height: 300px;
                padding: 10px;
                background-color: #cfcfcf;
                overflow-y: scroll;
            }
            input {
                width: 90%; 
                height: 30px;
            }
            button {
                margin-top: 10px;
                width: 90%;
                height: 30px;
            }
        </style>
    </head>
    <body>
        <pre id="messages"></pre>
        <input type="text" id="message" />
        <!-- Clicking the button will call the window.send function -->
        <button onclick="window.send()">send</button>
    </body>
    <script src="./index.js"></script>
</html>
```

The last thing we have to do is to hook up the user interface to the fish. We want to

1. show all chat messages, i.e. fish's state in the `pre` element
1. send out a chat message event when the user clicks the _Send_ button

In the `index.ts` file, add the following code:

```ts
Pond.default().then(pond => {
    // Observe our chat fish. This means that our callback function will
    // be called anytime the state of the fish changes
    pond.observe(ChatFish, state => {
        // Get the `pre` element and add all chat messages to that element
        const messages = document.getElementById('messages')
        messages.innerHTML = state.join('\n')
        // Scroll the element to the bottom when it is updated
        messages.scrollTop = messages.scrollHeight
    });

    // This function will be called whenever the user clicks 'send'
    (window as any).send = () => {
        // Get the text written in the input field
        const message = (document.getElementById('message') as HTMLTextAreaElement).value
        document.getElementById('message').innerHTML = ''
        // Send the message to a stream tagged with our chat tag
        pond.emit(chatTag, message)
    }
})
```

To test that everything works navigate to the `chat` directory, run `npm run start` and open (http://localhost:1234). You should see the chat app. You won't yet be able to send messages since ActyxOS isn't running on your device.

### Package and run our app

In order to run the chat app on our Android devices, we need to package and deploy it.

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

Before packaging the app, run `npm run build` to create a distribution version of the web app (which will be placed in the `dist` directory). If everything works well you should see something like this:

![](/images/tutorial/npm-run-build.png)


Now use the ActyxOS Node Manager to package and deploy the app (use the path to the `chat` directory) to both devices:

![](/images/tutorial/node-manager-package-app.png)

![](/images/tutorial/node-manager-deploy-app.png)

If you open ActyxOS on the Android devices, you should now see the chat app. You can click and open the app and should now be able to chat 😀!

![](/images/tutorial/find-open-use-chat-app.png)

You have just built a multi-node application that would traditionally have required application web server, a central replicatd database, possibly a pub-sub broker, etc. Here is what you can try out:

- Do the chat messages get shared between nodes?
- Do I see history if I restart the app?
- What happens if I chat or restart in airplane mode?
- How are messages sorted after I reconnect a device?

We hope that you are now starting to experience the power of decentral compusing and the Actyx Platform. If you are keen to dive a bit deeper, check out the following further resources.

## Further resources

- Learn more about [ActyxOS](./os/introduction.md)
- Learn more about [Actyx Pond](./pond/getting-started.md)
- Check out the additional libraries and tools such as the
   - The Actyx Pond [VS Code extension](https://marketplace.visualstudio.com/items?itemName=Actyx.actyx-pond) for efficiently writing fishes
   - The [React-Pond](https://github.com/actyx-contrib/react-pond) library to quickly integrate with React
   - ActyxOS SDKs for [Rust](./os/sdks/rust.md) and [JS/TS](./os/sdks/js-ts.md)


:::note Join our Discord chat
Feel free to join our [Actyx Developer Chat](https://discord.gg/262yJhc) on Discord. We would love to hear about what you want to build on the Actyx platform.
:::
