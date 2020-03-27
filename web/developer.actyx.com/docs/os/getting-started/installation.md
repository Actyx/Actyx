---
title: Installation
---

ActyxOS can be installed on either Docker or Android.

## Installing on Android

In order to install ActyxOS on an Android device you need:

- a device running Android 6.0 or above with at least 4GB of RAM
- [`adb`](https://developer.android.com/studio/command-line/adb) running on your machine (see [this installation guide](https://www.xda-developers.com/install-adb-windows-macos-linux/))

Download the latest version of the ActyxOS APK from https://downloads.actyx.com and install the APK on your Android device using [`adb`](https://developer.android.com/studio/command-line/adb):

```
adb install axos.apk
```

ActyxOS should now be installed and running on your Android device.

## Installing on Docker

In order to install ActyxOS on a Docker host you will need to have a working installation of Docker (see [the installation documentation](https://docs.docker.com/install/)).

ActyxOS is published on DockerHub, so start the image as follows:

```
docker pull actyx/os
docker run -it --rm -v actyxos-data:/data --privileged --network=host actyx/os
```

ActyxOS should now be running in your Docker environment.


## Communicate with the node

In order to check on its status and interact with the node, you need to download the Actyx CLI (`ax` or `ax.exe`) from https://downloads.actyx.com and add it to your path.

You can then check on your ActyxOS node:

```
ax nodes ls --local <DEVICE_IP>
```

Please refer to the [Actyx CLI documentation](/docs/cli) to learn more about using the Actyx CLI.

## Deploy and start an app

You can deploy and start apps using the Actyx CLI you previously installed. To see how this works, download the two example apps for https://github.com/actyx/sample-apps:

```
git clone https://github.com/actyx/sample-apps
cd sample-apps/
```

Inside the `sample-apps` directory will find
- sample node settings: `node-settings.yml`
- a directory with a _WebView_ app: `webview-app`
- a directory with a _Docker_ app: `docker-app`

First, we must configure the node using the sample settings:

```
ax settings set --local com.actyx.os @node-settings.yml <DEVICE_IP>
```

Depending on whether we are running ActyxOS on Android or on Docker, we now package and deploy either the _WebView App_ (for Android) or the _Docker App_ (for Docker). Using Android as an example, we do the following:

```
cd webview-app/
ax apps package manifest.yml
ax apps deploy --local com.actyx.sample-1.0.4.tar.gz
```

Finally, we can start the app with the following command.

```
ax apps start --local com.actyx.sample <DEVICE_IP>
```

Verify that everything is working, as follows:

```
ax apps ls --local <DEVICE_IP>
```

## Problems?

Ask for help on on [our GitHub repository](https://github.com/actyx/sample-apps) or [Twitter](https://twitter.com/actyx).

## Learn more

Jump to the different _Guides_ to learn more about the different aspects of ActyxOS.


