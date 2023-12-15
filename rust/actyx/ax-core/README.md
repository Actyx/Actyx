[![Latest Version](https://img.shields.io/crates/v/ax_core.svg)](https://crates.io/crates/ax_core)
[![Rust Documentation](https://docs.rs/ax_core/badge.svg)](https://docs.rs/ax_core)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> AX Core

[AX](https://crates.io/crates/ax) is a decentralized event database, streaming and processing
engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/)
apps.
It makes it easy to run distributed applications on multiple nodes.
It is a piece of software that allows you to run your own apps on one or more edge devices and have these apps seamlessly communicate and share data with each other.

---

> **Note:** _AX has been created by the company [Actyx AG](https://developer.actyx.com/) and predates the unrelated [Actix framework](https://crates.io/crates/actix) (although AX wasn’t open-sourced until Oct’23).
> While we have changed crate names to avoid confusion, the company name remains Actyx AG and appears in some places._

---

This crate contains the implementation of the AX functionality, which will eventually be shaped such that it can be embedded by another application.
For now, its intended purpose is only to serve as a dependency of the [AX crate](https://crates.io/crates/ax).
