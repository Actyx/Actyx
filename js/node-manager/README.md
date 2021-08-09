# Actyx Node Manager

The Node Manager is a graphical (Electron-based) tool for remotely managing Actyx nodes.

![Actyx Node Manager](https://developer.actyx.com/assets/images/node-overview-f963e8e2a9f2ba389003c40ee7161d81.png)

## Features

- View status of nodes (connected to using IP:PORT)
- View and edit node settings
- Create user keypairs for admin authentication
- Sign JSON-based app manifests
- Generate swarm keys
- Diagnostics (swarm connectivity, event offsets)

## Structure and architecture

### Code

Here are the key folders and files:

```lang-none
src/                 # contains the Electron app code
  |-- client/        # client (webview) React application
  |-- node/          # node-side code (incl. IPC setup)
  |-- common/        # shared types and utils
  |-- index.ts       # app setup and launch code
native/              # native Rust code for interacting with Actyx nodes
forge.config.js      # Electron packaging configuration
tailwind.config.js   # config of tailwind CSS framework
```

### Rust-based native addon

A native Node.js module written in Rust using [Neon](https://github.com/neon-bindings/neon) provides a couple of key functions to the Node.js process for interacting with Actyx nodes.

In the `/src/node/native` folder, a typescript definition files describes the functions made available by the native module.

Build the native bindings using `npm run build-native-bindings` before running or building the Node Manager.

### Electron app

The Electron app is composed of a Node.js process and a web-based client running in the Electron webview (or whatever it's called). The web-client uses the Electron IPC functionality to communicate with the Node.js process. This is used, for example, to access the native module.

The web-based client is a simple React app without anything fancy. It was designed from the ground up with global state in mind so as to offer a fluid user-experience to the user.

## Development

Use `nvm use` to setup the correct Node.js version for development (and building/packaging). Alternatively, install the correct Node.js version yourself (see `.nvmrc`).

Key scripts:

- `npm i` install dependencies
- `npm run dev` start the application in dev mode
- `npm run build` to build the node manager
- `npm run build-native-bindings` to build native dependencies
- `npm run build-electron-webpack` to build the electron app
- `npm run dist` create a distribution package

## Distribution

The Node Manager can be packaged for macOS, Windows, Debian-based, and Fedora-based Linux distributions. You can build a distribution version for your current platform using `npm run dist`. Cross-building is currently done in CI.
