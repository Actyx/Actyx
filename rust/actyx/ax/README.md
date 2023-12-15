[![Latest Version](https://img.shields.io/crates/v/ax.svg)](https://crates.io/crates/ax)
[![Rust Documentation](https://docs.rs/ax/badge.svg)](https://docs.rs/ax)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> AX Databank and CLI

[AX](https://crates.io/crates/ax) is a decentralized event database, streaming and processing
engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/)
apps.
It makes it easy to run distributed applications on multiple nodes.
It is a piece of software that allows you to run your own apps on one or more edge devices and have these apps seamlessly communicate and share data with each other.

---

> **Note:** _AX has been created by the company [Actyx AG](https://developer.actyx.com/) and predates the unrelated [Actix framework](https://crates.io/crates/actix) (although AX wasn’t open-sourced until Oct’23).
> While we have changed crate names to avoid confusion, the company name remains Actyx AG and appears in some places._

---

## Installation

This is a binary crate, its intended use is via

    cargo install ax

The installed executable hosts both the databank and command line interface components.

## Running the Databank

The AX node offering data services is started by running

    ax run

which will look for (or initialize) the directory `./ax-data` that holds both service configuration and user data.
You are invited to use the `--help` command line option to learn more about startup options (like choosing a different storage folder or TCP ports to listen on).
Please refer to the [AX user documentation](https://developer.actyx.com/docs/reference/actyx-reference) for more details.

## Using the CLI

Besides the services, which are typically run in the background, AX offers various tools for managing running AX nodes (also over the network), cryptographic key material, and interacting with the stored event data.
For more details check the output of

    ax --help

or the [AX developer documentation](https://developer.actyx.com/docs/reference/cli/cli-overview).
