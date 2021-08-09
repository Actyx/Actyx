# Actyx Cosmos

— _Everything you ever wanted._

Actyx is a **decentralized** event **database**, **streaming** and **processing** engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/) apps.
For more information on how to use it, please refer to [developer.actyx.com](https://developer.actyx.com).
You’re also very welcome to drop us a line in [the forum](https://community.actyx.com/).

## How to get Actyx

For up-to-date binaries built for a variety of host systems please refer to [Actyx Releases](https://developer.actyx.com/releases).
If you want to build it yourself from source, you’ll need GNU make (version 4.2 or later).

Common commands are:

```sh
# compile Actyx and Actyx CLI for your current host system
make current

# build all Typescript and .Net libraries
make all-js all-dotnet
```

There are provisions in the Makefile for cross-building, but those currently only work for Actyx employees — we’ll fix that soon.

## Contributing

We welcome all kinds of contributions, whether they are in the form of bug reports, feature requests, or code.
Just open an issue or pull request and we’ll guide you through the process where needed.

Please be respectful when interacting with others on this repository.
We reserve the right to ban you from participating in discussions or development in case of repeated or severe cases of uncivilised conduct.

## Licensing

Actyx, Actyx CLI, Actyx Node Manager, and the SDKs (for Typescript/Javascript, C#, Rust) are available under the [Apache 2.0](LICENSE.Apache_2.0) open-source license. This license and the accompanying [NOTICE](NOTICE) applies to all files in the `dotnet`, `integration`, `js`, `jvm`, `rust`, `third-party`, `wix` directories except where specified otherwise.

For commercial licensing please contact [Actyx Support](https://www.actyx.com/enterprise).
