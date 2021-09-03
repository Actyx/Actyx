# todo-react with local-first cooperation

_(This is a fork from `mdn`, see there for the [React-focused README](https://github.com/mdn/todo-react/blob/master/README.md))_

**Demonstration of peer-to-peer collaborative editing!**

This adaptation of the classic To-Do list example demonstrates [local-first cooperation](https://www.local-first-cooperation.org/) using [Actyx](https://developer.actyx.com/).
The idea is that persistence and business logic are **executed locally**, based on events stored in Actyx.
Multiple nodes in the same network will find each other using mDNS, making the TODO list **collaborative**.

You are very welcome to check out [this pull request](https://github.com/actyx-contrib/todo-react/pull/5/files) to convince yourself that this is neither daunting nor magical — it is an application of [event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) to distributed peer-to-peer applications.

## Trying it out

After cloning this repository, do the following inside the new directory (you’ll need at least node.js 10):

```bash
npm install --global yarn
yarn install
```

### Start Actyx

Before starting the app, you’ll need to have Actyx running, so get the latest version from [here](https://developer.actyx.com/releases/actyx/latest).

On **Windows**, it suffices to run the MSI installer, Actyx will automatically be started as a service. Note that the installer is not signed, so you'll need to allow it to run explicitly.

On **Mac or Linux**, you’ll need to make the downloaded binary executable and start it;
since it will store some data in its current working directory, you may want to do this from this project’s folder.

On **Mac** you'll need to allow it to run:

* Make it executable: `chmod +x actyx`
* Start it once and cancel the warning that pops up
* Go to `System Preferences -> Privacy & Security`
* Select `Allow Anyway` beside the "actyx was blocked ..." message
* Start it again
* Select `Open`

For **Android** please download the latest version from [here](https://developer.actyx.com/releases/actyx/latest) and install the APK with `adb install actyx.apk`, then start the Actyx app on your phone or tablet.

For more information on how to install and run Actyx on the different platforms, refer to [this installation guide](https://developer.actyx.com/docs/how-to/local-development/install-actyx).

### Start the app

Now you can start the app:

```bash
yarn start
```

For Android, it is easiest to serve the web app from your computer and open it in the browser of your tablet or phone:

```bash
yarn build
yarn global add serve
serve -s build
# note down the external IP:port of your computer from the above output
adb shell am start -a android.intent.action.VIEW -d http://<IP:port>
```

This should open a browser tab in which — after a few seconds of [create-react-app](https://reactjs.org/docs/create-a-new-react-app.html) magic — you’ll see an empty TODO list.
Don’t be shy, try it out!

### ‼️ Automatic synchronisation between nodes ‼️

The real deal is to use another computer or tablet in the same network:
Once you start Actyx and the app there, you should see both screens synchronised (it may take a few seconds at first while Actyx uses mDNS to find other nodes).

This communication is all purely local, no cloud services or relays are involved. You can try disabling network connections (e.g. by switching off wifi), making modifications, then switching the wifi back on and violà: data will be synchronised!
(It may take up to 30sec for the synchronisation to resume, since by default Actyx announces its synchronisation state only at that interval.)

## If the two nodes don’t sync

Depending on your network setup, it may not be possible for the Actyx nodes to discover each other.
Due to how containers work, this will always be a problem in Docker, lxc, etc.
If your nodes do not seem to synchronise, refer to [this guide on synchronization errors](https://developer.actyx.com/docs/how-to/troubleshooting/node-synchronization).

_(You can also replace `localhost` in any of the above with the IP of another Actyx node, e.g. your tablet, to perform remote management.)_
