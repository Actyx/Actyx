---
title: Quickstart
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Let's jump right in and get a first distributed application up and running.

:::warning Work in Progress
This guide is a work in progress. We appreciate your feedback, so please [let us know](#troubleshooting) if you have any issues.
:::

## Requirements

- **Git**, which you can [install from here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
- **Docker**, which you can [install from here](https://docs.docker.com/install/)
- **Node.js** and **npm**, which you can [install from here](https://nodejs.org/en/)
- A second device in your network that is running either Android or Docker
- `adb` (if you’ll use Android devices), which can be installed according to [this guide](https://www.xda-developers.com/install-adb-windows-macos-linux/)


## Prepare

All the files you need for this quickstart guide can be found in a [Github repository](https://github.com/Actyx/quickstart). Go ahead and clone it:

```
git clone https://github.com/Actyx/quickstart
```

Inside the newly created `quickstart` directory you should now find the following files and directories:

```
quickstart/
|--- scripts/
|--- sample-webview-app/
|--- sample-docker-app/
|--- misc/
```

## The business logic

ActyxOS is all about distributed apps communicating with one another, so let’s write an app that sends
events around and displays events from other apps. The easiest approach is to use
the [Actyx Pond](pond/introduction.md) library and write the app in the [Typescript](https://www.typescriptlang.org/) language. The distributable pieces of app
logic are called _fishes:_

```typescript
import { Pond, Semantics, OnStateChange, Subscription, FishTypeImpl } from '@actyx/pond'

// Each fish keeps some local state it remembers from the events it has seen.
// In this case, we’ll just remember some details of the latest event.
type State = { time: string, name: string, msg: string, } | undefined

const ForgetfulChatFish: FishTypeImpl<State, string, string, State> = FishTypeImpl.of({
    // The kind of fish is identified by the meaning of its event stream, the semantics
    semantics: Semantics.of('ForgetfulChatFish'),

    // When the fish first wakes up, it computes its initial state and declares which
    // event streams to listen to. Most fishes subscribe to their own event stream.
    initialState: (_name, _sourceId) => ({
        state: undefined, // start without information about previous event
        subscriptions: [Subscription.of(ForgetfulChatFish)] // all fish of this kind
    }),

    // Upon each new event, keep some details of that event in the state.
    onEvent: (_state, event) => ({
        time: new Date(event.timestamp / 1000).toISOString(),
        name: event.source.name,
        msg: event.payload
    }),

    // Show the state computed above to the outside world (see Pond.observe below).
    onStateChange: OnStateChange.publishPrivateState(),

    // Upon each received command message generate a single event.
    onCommand: (_state, msg) => [msg],
})
```

This piece of logic can be run on multiple edge devices, each running an ActyxOS node, and we’ll do so in the following.
But before we can do that we need to add some code that takes the type of fish defined above and wakes up one specific
instance, identified by its name.

```typescript
(async () => {
    // get started with a Pond
    const pond = await Pond.default()
    // figure out the name of the fish we want to wake up
    const myName = process.argv[2] || pond.info().sourceId
    // wake up fish with the given name and log its published states
    pond.observe(ForgetfulChatFish, myName).subscribe(console.log)
    // send 'ping' a message every 5sec to generate a new event
    setInterval(() => pond.feed(ForgetfulChatFish, myName)('ping').subscribe(), 5000)
})()
```

This example shows how to start this fish and have it emit one event every five minutes.
Now we want to see this in action, so let’s install the necessary ingredients.

## Install the Actyx CLI

Download and install the latest version of the Actyx CLI (`ax`). You can find builds for several operating systems at https://downloads.actyx.com.

Once installed you can check that everything works as follows:

```
ax --version
```

:::tip Having trouble?
Check out the [troubleshooting section](#troubleshooting) below or let us know.
:::

## Start ActyxOS

Now, start ActyxOS as a Docker container on your local machine. Since ActyxOS is published on DockerHub, you can start it using the following command:

```
docker run -it --rm -e AX_DEV_MODE=1 -v actyxos_data:/data --privileged -p 4001:4001 -p 4457:4457 -p 4243:4243 -p 4454:4454 actyx/os
```

:::note
As you can see, you need to provide a persistent volume and set up some port forwarding. For more information about running ActyxOS on Docker or other hosts, please refer to the [ActyxOS documentation](os/getting-started/installation.md).
:::

Now that it is running, we need to provide the ActyxOS node with a couple of settings. These allow the node to function correctly. For now, we will just use the sample settings defined in `misc/local-sample-node-settings.yml`. Run the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
ax.exe settings set --local com.actyx.os @misc\local-sample-node-settings.yml localhost
```

</TabItem>
<TabItem value="unix">

```bash
ax settings set --local com.actyx.os @misc/local-sample-node-settings.yml localhost
```

</TabItem>
</Tabs>

😊 Congrats! Your computer is now running a fully configured ActyxOS node. You can check this by running

```
ax nodes ls --local localhost
```

## Run the app in Dev Mode

:::note
In the following we assume that you have cloned the [github repository with the sample apps](https://github.com/Actyx/quickstart) and opened a shell inside that folder.
:::

### Docker app

You’ll find the app prepared in the folder `sample-docker-app`. Inside this folder, run the following to install the dependencies:

```
npm install
```

Now you can start the app by saying

```
npm start Dori
```

This will connect to ActyxOS and then start printing out lines after a few seconds, corresponding to state updates from the ForgetfulChatFish named “Dori”.

### WebView app

The WebView app is prepared in the folder `sample-webview-app`. As for the docker app, first install the dependencies:

```
npm install
```

Then start the build-in webserver by saying

```
npm start
```

The app itself will only start once you open it in your web browser, you should find it at http://localhost:1234 (or check the output of the above command).
If you kept the docker app running in your terminal, you should see its messages appear between the ones you can create by clicking the “send message” button.

:::tip
The fish we used here is called ForgetfulChatFish because it only remembers some details from the most recent event it has seen.
Why don’t you try your hand at keeping the last ten messages in its state and render that as a list in the UI?
:::

## Deploy the app

### ActyxOS on Docker

First, we need to build a docker image containing the app. This is done inside the `sample-docker-app` folder by running

```
npm run build:image
```

The resulting image is packaged into an Actyx App using the Actyx CLI:

```
ax apps package manifest.yml
```

After a few moments you’ll find an app package in your folder. This is deployed into the local ActyxOS node by saying

```
ax apps deploy --local com.actyx.sample-docker-app-1.0.0.tar.gz localhost
```

You can check the state of this app using

```
ax apps ls --local localhost
```

Before you can start the app, you’ll need to supply valid settings — in this example the empty object is enough:

```
ax settings set --local com.actyx.sample-docker-app '{}' localhost
```

Now the app is started with

```
ax apps start --local com.actyx.sample-docker-app localhost
```

If you still have the webview app open running in dev mode in your browser, you should see the ping messages appear in there. The two apps are so far served by the same ActyxOS node.
In order to make this sample fully distributed you can either start another ActyxOS node on a different computer (by repeating the ActyxOS steps above), or you can continue with an Android device.

### ActyxOS on Android

First, download the latest ActyxOS APK from https://downloads.actyx.com and install it using `adb`:

```
adb install actyxos-1.0.0.apk
```

Start ActyxOS by clicking on the ActyxOS app in Android.

:::tip Having trouble installing?
Check out the [troubleshooting tips](#troubleshooting) below and the [ActyxOS installation guide](./os/getting-started/installation.md).
:::

Now that you have installed ActyxOS on the second device, let's configure the node and then package and deploy one of the sample apps. From the `quickstart` folder, run the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
ax.exe settings set --local com.actyx.os @misc\remote-sample-node-settings.yml <DEVICE_IP>
```

</TabItem>
<TabItem value="unix">

```bash
ax settings set --local com.actyx.os @misc/remote-sample-node-settings.yml <DEVICE_IP>
```

</TabItem>
</Tabs>

:::note
Replace `<DEVICE_IP>` with the IP of your Android device.
:::

The ActyxOS node on the second device should now be fully functional 😊!

Now go back to the `sample-webview-app` folder and create the production build for this web app:

```
npm run build
```

The resulting files in the `dist` folder are now packaged into an Actyx App bundle using

```
ax apps package manifest.yml
```

The resulting bundle is then deployed to the Android device by saying

```
ax apps deploy --local com.actyx.sample-webview-app-1.0.0.tar.gz <DEVICE_IP>
```

As for the docker app, you need to supply valid settings:

```
ax settings set --local com.actyx.sample-docker-app '{}' <DEVICE_IP>
```

Then the app can be started, either by selecting it from the ActyxOS app on Android or by using the CLI:

```
ax apps start --local com.actyx.sample-webview-app <DEVICE_IP>
```

Congratulations, you have just packaged and deployed an ActyxOS app to a remote ActyxOS node!

You should now see two apps running locally on you computer and the app running on the device communicating with each other without any central servers or databases.

This brings us to the close of this quickstart guide.

## Further reading

- Learn more about ActyxOS and how to use it in the [ActyxOS docs](/docs/os/getting-started/installation)
- Dive into the Actyx Pond and its fishes in the [Actyx Pond docs](/docs/pond/getting-started/installation)

## Troubleshooting

### I can't get it to work

Please get in touch with us at developers@actyx.com and we will get back to you as soon as possible.
