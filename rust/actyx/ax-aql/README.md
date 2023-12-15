[![Latest Version](https://img.shields.io/crates/v/ax_aql.svg)](https://crates.io/crates/ax_aql)
[![Rust Documentation](https://docs.rs/ax_aql/badge.svg)](https://docs.rs/ax_aql)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> AX AQL

[AX](https://crates.io/crates/ax) is a decentralized event database, streaming and processing
engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/)
apps.
It makes it easy to run distributed applications on multiple nodes.
It is a piece of software that allows you to run your own apps on one or more edge devices and have these apps seamlessly communicate and share data with each other.

---

> **Note:** _AX has been created by the company [Actyx AG](https://developer.actyx.com/) and predates the unrelated [Actix framework](https://crates.io/crates/actix) (although AX wasn’t open-sourced until Oct’23).
> While we have changed crate names to avoid confusion, the company name remains Actyx AG and appears in some places._

---

This crate defines the syntax of the AQL query language.
It should in most cases be used through the [AX SDK](https://crates.io/crates/ax_sdk).
