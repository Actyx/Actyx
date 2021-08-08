# Actyx Node Manager

## Introduction

### Code structure

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

Neon builds the native module and creates the `/native/index.node` module file. This gets packaged into the Electron app by Webpack.

Note that building the native module requires information about the Electron build environment. This is achieved using the `electron-build-env` package.

The native modules must be built using `npm run build-native` before packaging the Electron app.

### Electron app

The Electron app is composed of a Node.js process and a web-based client running in the Electron webview (or whatever it's called). The web-client uses the Electron IPC functionality to communicate with the Node.js process. This is used, for example, to access the native module.

The web-based client is a simple React app without anything fancy. It was designed from the ground up with global state in mind so as to offer a fluid user-experience to the user.

## Development

Use `nvm use` to setup the correct Node.js version for development (and building/packaging). Alternatively, install the correct Node.js version yourself (see `.nvmrc`).

Scripts:

- `npm i` install dependencies
- `npm run build-native` to build native dependencies
- `npm run make` will create a distributable for the application based on forge config
- `npm run start` start the application in dev mode
- `npm run lint` validate code with TypeScript and ESLint
- `npm run lint:fix` validate and fix code using TypeScript and ESLint
- `npm run clean` clean up files related created with make or publish
- `npm run check-deps` check for circular dependencies issues in the project
- `npm run build-prod` perform all lint and other checks and make a production build
- `npm run tsc` compile TypeScript
- `npm run tsc:watch` copile TypeScript in watch mode
- `npm run test` run all tests end-to-end included
- `npm run test-source` run unit/integration tests
- `npm run test-source:watch` run unit/integration tests in watch mode
- `npm run test-e2e` run end-to-end integration tests, on your dev machine this requires you to have built and installed the app in a specific location (look at `getAppPathForCurrentPlatform()` for more details). You can use `bin/prepare-test-e2e-mac.sh` to automatize some steps on MacOS.

## Build

To build and package run `npm run build-native` and then `npm run make`. This results in the following packaged being built:

- DMG for macOS
- Squirrel for Windows
- DEB for Debian-based distros
- RPM for Redhat-based distros

All these will be built for the architecture on which the build/packaging is performed.

## Leftover notes from ActyxOS Node Manager

---
**BUG in build tool:** Build rpm is not working with: `rpmbuild --version >= 4.15.0`

The fix is already on master and waits for the next npm publish.

Temporary local fix:

After you execute `npm install`, goto `node_modules/electron-installer-redhat/src/dependencies.js:30` and change:

- ```return rpmVersionSupportsBooleanDependencies(output.trim().split(' ')[2])```

to

- ```return rpmVersionSupportsBooleanDependencies(output.trim().split(' ')[1])```

---

The build process is currently manual:

- Pull from docker the right version of ActyxOS
- Run `npm run clean` and `npm run make -- --platform {linux|win32|darwin}`. The result of
  the build will be written to the `out` folder. If the `--platform` parameter is omitted, the
  current platform will be assumend as the target platform.
