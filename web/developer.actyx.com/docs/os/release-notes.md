---
title: Release notes
---

This page contains information about new features, bug fixes, deprecations and removals in ActyxOS releases. For a more extensive description of the changes, check out our [blog](https://www.actyx.com/news/).

<!-- markdownlint-disable MD024 -->

## ActyxOS 1.1.0

Release date: 7.12.2020

Download:

- [Docker Hub](https://hub.docker.com/r/actyx/os)
- [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android)
- [Actyx Downloads page](https://downloads.actyx.com/)

### New features

- Introduced [ActyxOS on Linux in beta version](advanced-guides/actyxos-on-linux.md)
- ActyxOS nodes start up without the need to set settings by having default settings for all required values. You can find the new node settings schema [here](api/node-settings-schema.md)
- Many performance improvements

### Bug fixes

- Many stability improvements around our core infrastructure
- fixed an issue that resulted in a node crash if there are gaps in event offsets
- fixed an issue that led to ActyxOS on Android crashing if started without available network interface
- fixed an issue that resulted in default node settings not being used after manually set settings were unset
- changed the node settings schema so that properties with default values are not required anymore
- fixed an issue that led to ActyxOS not validating apps if optional properties in the manifest were missing (`description` and `settingsSchema`)
- fixed an issue that resulted in the node crashing if configured with an invalid bootstrap node address
- Fixed an issue that prohibited access to the [Console Service logging API](/os/api/console-service.md) from the browser and WebView Runtime due to CORS restrictions

## ActyxOS 1.0.0

Release date: 28.08.2020

Download:

- [Docker Hub](https://hub.docker.com/r/actyx/os)
- [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android)
- [Actyx Downloads page](https://downloads.actyx.com/)

### New features

- Introduced [ActyxOS on Windows in beta version](advanced-guides/actyxos-on-windows.md)
- Introduced ActyxOS on Android support for `armeabi-v7a` [ABI](https://developer.android.com/ndk/guides/abis.html#sa) devices
- Several performance improvements

### Bug fixes

- Significant stability improvements

## ActyxOS 1.0.0-rc.4

Release date: 03.07.2020

Download:

- [Docker Hub](https://hub.docker.com/r/actyx/os)
- [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android)
  
### Bug fixes

- Several internal stability and performance improvements
- Fixed a bug in the Docker Runtime that led to apps on the same node being stopped if they contain the same docker-compose.yml

## ActyxOS 1.0.0-rc.3

Release date: 12.06.2020

Download:

- [Docker Hub](https://hub.docker.com/r/actyx/os)
- [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android)

### New features

- Introduced configurable logging levels for ActyxOS nodes
  
### Bug fixes

- Several internal stability and performance improvements
  
## ActyxOS 1.0.0-rc.2

Release date: 25.05.2020

Download:

- [Docker Hub](https://hub.docker.com/r/actyx/os)
- [Google Play Store](https://play.google.com/store/apps/details?id=com.actyx.os.android)

### New features

- Introduced ActyxOS on Docker support for arm64v8 devices
- Removed the possibility to configure unknown scopes in ActyxOS node settings
- Added possibility to inline the settings schema into the app manifest
- Made the app icon property optional in the app manifest schema for webview apps
- Added automatic restart of all apps after shutdown of the ActyxOS node
- Added automatic restart of ActyxOS on Android after restart of Android
- Added system info section in ActyxOS on Android

### Bug fixes

- Fixed a bug that allowed apps to be running without valid node or app settings
- Fixed an issue that caused apps to be visible in the app switcher on Android after they were stopped
- Added proper handling of the back button on Android so that ActyxOS does not open the same app in multiple windows

### Removals and non-backwards compatible changes

- ActyxOS 1.0.0-rc.2 is only compatible with the Actyx CLI 1.0.0-rc.2 and will not work with apps that were packaged with earlier versions of the Actyx CLI

<!-- markdownlint-enable MD024 -->