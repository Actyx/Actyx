---
title: Quickstart
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Let's jump right in and get a first distributed application up and running.

## Requirements

- **Git**, which you can [install from here](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
- **Docker**, which you can [install from here](https://docs.docker.com/install/)
- **Node.js** and **npm**, which you can [install from here](https://nodejs.org/en/)
- A second device in your network that is running either Android or Docker
- `adb`, which can be installed according to [this guide](https://www.xda-developers.com/install-adb-windows-macos-linux/)


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
|--- sample-node-settings.yml
|--- package.json
```

## The business logic

ActyxOS is all about distributed apps communicating with one another, so let’s write an app that sends
events around and displays events sent around in this way by other apps. The easiest approach is to use
the Actyx Pond library and write the app in the Typescript language. The distributable pieces of app
logic are called _fishes_:

```typescript
import { Pond, Semantics, OnStateChange, Subscription, FishTypeImpl } from '@actyx/pond'

// Each fish keeps some local state it remembers from all the events it has seen
type State = { time: string, name: string, msg: string, } | undefined

const ForgetfulChatFish: FishTypeImpl<State, string, string, State> = FishTypeImpl.of({
    // The kind of fish is identified by the meaning of its event stream, the semantics
    semantics: Semantics.of('ForgetfulChatFish'),

    // When the fish first wakes up, it computes its initial state and event subscriptions
    initialState: (_name, _sourceId) => ({
        state: undefined, // start without information about previous event
        subscriptions: [Subscription.of(ForgetfulChatFish)] // subscribe across all names
    }),

    // Upon each new event, keep some details of that event in the state
    onEvent: (_state, event) => ({
        time: new Date(event.timestamp / 1000).toISOString(),
        name: event.source.name,
        msg: event.payload
    }),

    // Show the state computed above to the outside world (see Pond.observe below)
    onStateChange: OnStateChange.publishPrivateState(),

    // Upon each received command message generate one event
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
    // wake up fish of kind ForgetfulChatFish with name myName and log its published states
    pond.observe(ForgetfulChatFish, myName).subscribe(console.log)
    // send a message every 5sec to generate a new event
    setInterval(() => pond.feed(ForgetfulChatFish, myName)('ping').subscribe(), 5000)
})()
```

This example shows how to start this fish and have it emit one event every five minutes.
Now we want to see this in action, so let’s install the necessary ingredients.

## Install the Actyx CLI

Download and install the latest version of the Actyx CLI (`ax`). You can find different builds for your operating system at https://downloads.actyx.com.

Once installed you can check that everything works as follows:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
ax.exe --version
```

</TabItem>
<TabItem value="unix">

```bash
ax --version
```

</TabItem>
</Tabs>

:::tip Having trouble?
Check out the [troubleshooting section](#troubleshooting) below or let us know.
:::

## Start ActyxOS

Now, start ActyxOS as a Docker container on your local machine. Since ActyxOS is published on DockerHub, you can start it using the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
docker run -it --rm -e AX_DEV_MODE -v actyxos-data:/data  -p 4001:4001 -p 4457:4457 actyx/os
```

</TabItem>
<TabItem value="unix">

```bash
docker run -it --rm -e AX_DEV_MODE -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

:::note
As you can see, you need to provide a persistent volume and setup some port forwarding. For more information about running ActyxOS on Docker or other hosts, please refer to the [ActyxOS documentation](./os/getting-started/installation.md).
:::

Now that it is running, we need to provide the ActyxOS node with a couple of settings. These allow the node to function correctly. For now, we will just use the sample settings defined in `sample-node-settings.yml`. Run the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
ax.exe settings set --local @quickstart\sample-node-settings.yml 0.0.0.0
```

</TabItem>
<TabItem value="unix">

```bash
ax settings set --local @quickstart/sample-node-settings.yml 0.0.0.0
```

</TabItem>
</Tabs>

😊 Congrats! Your computer is now running a fully configured ActyxOS node.

## Start the sample app

Let's now run one of the sample apps you downloaded as part of the quickstart repository. We will start with a web-app defined in `webview-app/`. That directory should contain the following files:

```
sample-webview-app/
|--- app.ts
|--- index.html
|--- package.json
```

We will now build and start the app. The app will run locally on your computer and automatically connect to your ActyxOS node. Staying in the `quickstart` directory, now run the following command:

```bash
npm run sample-webview-app:start
```

Using your browser, you should now be able to access the app at http://localhost:8000.

## Let's decentralize

In order to experience the power of the Actyx Pond programming model, we will now start another instance of the app and see how these two instance will magically synchronize.

Start another instance, giving it a dedicated name and a dedicated port to bind to:

```bash
npm run sample-webview-app:start --name secondInstance --port 9000
```

You should now be able to access this instance at http://localhost:9000.

:::danger TODO
We probably need to explain what is happening here. I am not quite sure where that special code block (the aha block) goes.
:::

## Adding a second node

So far the two app instances have been communicating via the same local ActyxOS node you started on your computer. Let's now add another participant so you can see how ActyxOS allows these nodes to communicate peer-to-peer in your local network.

You can use either an Android device or a device running Docker (e.g. a RaspberryPi). Follow one of the two instructions sets below accordingly.

<Tabs
  defaultValue="android"
  values={[
    { label: 'Android device', value: 'android', },
    { label: 'Docker device', value: 'docker', },
  ]
}>
<TabItem value="android">

1. Download the latest ActyxOS APK from https://downloads.actyx.com
2. Install the APK using `abd` (see [this installation guide](https://www.xda-developers.com/install-adb-windows-macos-linux/))

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```bash
adb install actyxos-1.0.0.apk
```

</TabItem>
<TabItem value="unix">

```powershell
adb.exe install actyxos-1.0.0.apk
```

</TabItem>
</Tabs>

3. Start ActyxOS by clicking on the ActyxOS app in Android

</TabItem>
<TabItem value="docker">

1. Start the ActyxOS container on the device

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
docker run -it --rm -e AX_DEV_MODE -v actyxos-data:/data  -p 4001:4001 -p 4457:4457 actyx/os
```

</TabItem>
<TabItem value="unix">

```bash
docker run -it --rm -e AX_DEV_MODE -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

</TabItem>
</Tabs>

<br />

:::tip Having trouble installing?
Check out the [troubleshooting tips](#troubleshooting) below and the [ActyxOS installation guide](./os/getting-started/installation.md).
:::

Now that you have installed ActyxOS on the second device, let's configure the node and then package and deploy one of the sample apps. Configure the node using the provided second node settings file (`misc/second-node-settings.yml`):

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
ax.exe settings set --local @quickstart\mist\second-node-settings.yml <DEVICE_IP>
```

</TabItem>
<TabItem value="unix">

```bash
ax settings set --local @quickstart/misc/second-node-settings.yml <DEVICE_IP>
```

</TabItem>
</Tabs>

:::note
Replace `<DEVICE_IP>` with the IP of the second device.
:::

The ActyxOS node on the second device should now be fully functional 😊!

Now, let's package and install one of our sample apps. If you installed ActyxOS on Android use the `sample-webview-app`, if you installed on Docker use the `sample-docker-app`.

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows', value: 'windows', },
    { label: 'Linux/MacOS', value: 'unix', },
  ]
}>
<TabItem value="windows">

```powershell
# Package the app
ax.exe apps package sample-webview-app\manifest.yml
# Deploy the app
ax.exe apps deploy --local com.actyx.sample-webview-1.0.0.tar.gz <DEVICE_IP>
```

</TabItem>
<TabItem value="unix">

```bash
# Package the app
ax apps package sample-webview-app/manifest.yml
# Deploy the app
ax apps deploy --local com.actyx.sample-webview-1.0.0.tar.gz <DEVICE_IP>
```

</TabItem>
</Tabs>

Congratulations, you have just packaged and deployed an ActyxOS app to a remote ActyxOS node! On Docker, the app should now be running; on Android you should be able to start it from the ActyxOS app.

You should now see two apps running locally on you computer and the app running on the device communicating with each other without any central servers or databases.

This brings us to a close of this quickstart guide.

## Further reading

- Learn more about ActyxOS and how to use it in the [ActyxOS docs](/docs/os/getting-started/installation.md)
- Dive into the Actyx Pond and its fishes in the [Actyx Pond docs](/docs/pond/getting-started/installation.md)

## Troubleshooting

### I can't get it to work

Please get in touch with us at developer@actyx.com and we will get back to you as soon as possible.
