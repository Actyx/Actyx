---
title: Which programming languages does ActyxOS support?
sidebar_label: Supported languages
---

You can use any language you want with the [Docker Runtime](/os/docs/docker-runtime.html) and any transcompile-to-Javascript language with the [WebView Runtime](/os/docs/webview-runtime.html)

The Docker Runtime runs one or more docker containers on an edge device. You are free to build any docker image you want&mdash;if it builds, it runs.

For the WebView Runtime, you can use any language that transcompiles to Javascript. Examples include:

- [CoffeScript](https://coffeescript.org)
- [TypeScript](https://www.typescriptlang.org)
- [PureScript](http://www.purescript.org)
- [Scala.js](https://www.scala-js.org)
- C# with [JSIL](http://jsil.org/) or [Bridge.NET](https://www.scala-js.org)
- Java with [JSweet](http://www.jsweet.org) or [J2CL](https://github.com/google/j2cl)

Interacting with the [Event Service](/os/docs/event-service.html), the [Blob Service](/os/docs/blob-service.html) and the [Console Service](/os/docs/console-service) happens via their respective HTTP APIs. This means you can use any HTTP request library for interacting with the services from your language of choice.
