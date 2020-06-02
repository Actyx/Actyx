---
title: Actyx CLI release notes
---

This page contains information about new features, bug fixes, deprecations and removals in the Actyx CLI releases. For a more extensive description of the changes, check out our [blog](https://www.actyx.com/news/).

## Actyx CLI 1.0.0-rc.2

**Release date: 25.05.2020**

[Download](https://downloads.actyx.com/)

### New features

- Added functionality to package Docker apps for amd64 and arm64v8 if specified in the app manifest
- Introduced more understandable error messages
- Added option to format the output as JSON

### Removals and non-backwards compatible changes
- Removed the possibility to specify more than one node in all commands but `ax nodes ls`