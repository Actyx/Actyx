---
title: ActyxOS release notes
---

This page contains information about new features, bug fixes, deprecations and removals in ActyxOS releases. For a more extensive description of the changes, check out our [blog](https://www.actyx.com/news/).

## ActyxOS 1.0.0-rc.2

**Release date: 25.05.2020**

Download:
- [Docker Hub](https://hub.docker.com/repository/docker/actyx/os)
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