---
title: Release notes
---

This page contains information about new features, bug fixes, deprecations and removals in the Actyx CLI releases. For a more extensive description of the changes, check out our [blog](https://www.actyx.com/news/).

## Actyx CLI 1.0.0

Release date: 28.08.2020

[Get it from our Downloads page](https://downloads.actyx.com/)

### New features

- Introduced support for `arm` and `armhf` devices

### Bug Fixes

- Several stability improvements
- Fixed a bug that did not allow you to inline the settings schema in the app manifest

## Actyx CLI 1.0.0-rc.3

Release date: 12.06.2020

[Download](https://downloads.actyx.com/)

### New features

- On ActyxOS on Docker and on Android, all node and app logs are now available via `ax logs tail`
- Added an installer for the Actyx CLI for Windows

### Bug Fixes

- Fixed a bug that prevented packaging docker apps containing a "/" in their name
- Fixed a bug where inlining the schema would corrupt the app package
- Improved application packaging times

## Actyx CLI 1.0.0-rc.2

Release date: 25.05.2020

[Download](https://downloads.actyx.com/)

### New features

- Added functionality to package Docker apps for amd64 and arm64v8 if specified in the app manifest
- Introduced more understandable error messages
- Added option to format the output as JSON

### Removals and non-backwards compatible changes

- Removed the possibility to specify more than one node in all commands but `ax nodes ls`
