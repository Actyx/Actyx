---
id: webview-runtime
title: WebView Runtime
permalink: os/docs/webview-runtime.html
next: docker-runtime.html
---

The **WebView Runtime** allows you to easily run web apps.

> Private beta
> 
> The WebView Runtime is currently in private beta with select users. It is planned for **public release in Q4 2019** (see the [Q4/19 Roadmap Update](/blog/2019/09/18/Q4-19-roadmap-update.html) for more information). To stay up to date about upcoming releases please check out our [blog](/blog), where we post release notes and roadmap updates.

## Contents

- [Overview](#overview)
- [Basics](#basics)
    - [Web apps](#web-apps)
    - [Multi-tasking by end-users](#multi-tasking-by-end-users)
- [Usage](#usage)
    - [Building web apps](#building-web-apps)
    - [Packaging web apps](#packaging-web-apps)
    - [Deploying web apps](#deploying-web-apps)
    - [Monitoring web apps](#monitoring-web-apps)

## Overview {#overview}

The WebView runtime is a service that reliably runs [single-page applications (SPAs)](#single-page-applications)&mdash;we call them _web&thinsp;apps_&mdash;built on HTML, JavaScript, and CSS on edge devices. It is the basis for developing interactive UI-based apps for end-users.

Key capabilities:

- Reliably serve web apps and assets in platform-specific webview container
- Provide access to auxiliary ActyxOS services (e.g. [Event Service](/docs/os/event-service.html))

## Basics {#basics}

### Web apps {#web-apps}

ActyxOS web apps are [single-page applications (SPAs)](https://en.wikipedia.org/wiki/Single-page_application) in which pages are dynamically generated on the client-side depending on user interactions. They are traditionally built as a combination of HTML, JavaScript, and CSS.

### Multi-tasking by end-users {#multi-tasking-by-end-users}

The WebView runtime supports multi-tasking. This means that end-users can have multiple apps running simultaneously and can switch between them. Please refer to the specific host platform for more information about how this works. For Android, for example, refer to the [Android Documentation](https://support.google.com/android/answer/9079646?hl=en).

## Usage {#usage}

### Building web apps {#building-web-apps}

You can build your web app from scratch or use any of the common JavaScript frameworks or libraries. The most popular frameworks/libraries are:

- [React.js](https://reactjs.org),
- [Vue.js](https://vuejs.org),
- [AngularJS](https://angularjs.org),
- [Ember.js](https://emberjs.com),
- [Sencha Ext JS](https://www.sencha.com/products/extjs/); and,
- [Knockout](https://knockoutjs.com).

> Recommendation: ReactJS
>
> Unless you are very proficient in another framework, we recommend starting with [ReactJS](https://reactjs.org) using the [`create-react-app`](https://github.com/facebook/create-react-app) command.

### Packaging web apps {#packaging-web-apps}

The [Actyx CLI](/os/docs/actyx-cli.html) provides the `ax apps package` command for packaging ActyxOS apps for deployment. Based on the app's manifest (a YAML file), the CLI will automatically validate app properties and assets, and, finally, generate a tarball for deployment.

For web apps, the manifest should have the following structure. For a complete schema check out the [App Manifest Schema](/os/docs/app-manifest-schema.html).

```yaml
manifest-version: "1.0" # The version of the manifest
type: web # The type of app this is (web or docker)
id: com.example.app1 # A unique app id
version: 1.0.3 # The version (semantic versioning)
display-name: App 1 # A human-friendly display name
description: "A great first app" # A short description
icon: ./build/assets/app-icon.png # Path to an app icon to use
dist: ./build/ # Path to the app assets
main: ./build/index.html # Path to the index HTML page
settings-schema: ./settings-schema.json # Path to the app's settings schema
```

The `dist` property should point to a directory at which the app's files and assets are located. Here is an example of the project directory based on the above manifest:

```
app1/
    ax-manifest.yml
    package.json
    settings-schema.json
    src/
        <source files>
    build/
        index.html
        bundle.js
        styles.css
        assets/
            logo.png
            app-icon.png
```

> Note
>
> The `dist` directory is usually created automatically by your build process. It is rarely the same as your `src` directory. Please refer to your environment's or framework's instructions for building your app for distribution.

With the above manifest and the given directory structure, you can package your app using the `ax apps package` command. The [Actyx CLI](/os/docs/actyx-cli.html) will automatically validate and analyze the manifest, package necessary files and create a tarball for deployment.

```bash
# Go to the project directory
$ cd app1/
# Package the app
$ ax apps package ax-manifest.yml
> Packaging web app...
> com.example.app1 (1.0.3) successfully packaged: com.example.app1-1.0.3.tar.gz
```

### Deploying web apps {#deploying-web-apps}

The [Actyx CLI](/os/docs/actyx-cli.html) provides the `ax apps deploy` command for deploying apps to edge devices. The CLI will automatically read the manifest file and deploy the built package to the edge device.

> Local deployments only
>
> Currently, the [Actyx CLI](/os/docs/actyx-cli.html) only supports local interaction with devices (using the `--local` flag). We plan to release remote deployment functionality in 2020. Please check out our [blog](/blog) for release updates.

The following example shows how you can deploy an app to a node accessible in the local area network:

```bash
# Go to project directory
cd app1/
# Deploy the app
$ ax apps deploy --local com.example.app1-1.0.3.tar.gz 10.7.0.32
> Deploying web app...
> com.example.app1 (1.0.3) (com.example.app1-1.0.3.tar.gz) successfully deployed to node at 10.7.0.32.
```

### Monitoring web apps {#monitoring-web-apps}

A web app may generate logs using the global [console](https://developer.mozilla.org/en-US/docs/Web/API/console) object. These log messages are automatically captured by ActyxOS and made available to you for monitoring and debugging.

You can access or tail (use the `--tail` flag) these logs using the [Actyx CLI](/os/docs/actyx-cli.html) as shown in the following example.

```bash
$ # run `ax logs tail --help` for more information on the command
$ ax logs tail --local 10.7.0.32
> com.example.app1-1.0.3::console | 2019-09-11T21:46:12.106Z [info] Starting app...
> com.example.app1-1.0.3::console | 2019-09-11T21:46:12.113Z [debug] Setting route '/activities'
> com.example.app1-1.0.3::console | 2019-09-11T21:46:12.113Z [debug] 34 activities loaded
> com.example.app1-1.0.3::console | 2019-09-11T21:46:12.113Z [info] User 'jdoe' registered
> com.example.app1-1.0.3::console | 2019-09-11T21:46:12.114Z [debug] Setting route '/preferences'
```

### Undeploying web apps {#undeploying-web-apps}

Undeploying an app means deleting it from the device. This can be done with the `ax apps undeploy` command.

Example:

```bash
# Undeploy an app
$ ax apps undeploy --local com.example.app1 10.7.0.32
> Undeploying app `com.example.app1`...
> App 'com.example.app1' (1.0.3) successfully undeployed from 10.7.0.32
```
