# Versioning Ax and friends

In this document there will be three main "players" (as of writing, the names are not
fixed), I start by introducing them and their relevance to the project, then proceed
to explain the considered solution.

## "Players"

The three main players are:
- `ax` - the result of merging 3 tools:
  - the previously existing `ax` CLI tool,
    which handled several aspects of running an Actyx swarm;
  - the `actyx` binary, which was the system that ran "as a node" in the swarm,
    processing events, handling queries, etc;
  - the `cert` tool, which was used for licensing purposes and complementing an Actyx
    swarm's authorization and authentication capabilities.
- `ax-core` - the library resulting of extracting the core functionality from the
  `actyx` binary such that it can be exposed for other uses, such as being embedded
  in an application
- `node` - the node component from `actyx`, which can be thought of as the inner Actyx
  "stack", the main difference between the `node` and `ax-core` is precisely the reason
  behind this document as it should become evident

## The Problem

Before "the merge", `actyx` and `ax` were separate tools which ran completely distinctly
from each other. Post "merge" `actyx` was converted into the command — `ax run`.

As they were separate tools, they were versioned separately; at the time of writing,
`ax` has the version `3.2.1` and `actyx` the version `2.17.0`.

It is clear they have very different versions and this is where the problem starts.

By SemVer's definition, the CLI did not suffer a change dramatic enought to warrant a
breaking change, but at the same time, it is no longer the same tool users are used to.

`actyx` itself did not suffer major changes either, we deprecated some platforms but
that does not constitute a "breaking change" in interface terms; the functionality
itself also did not change dramatically enough to warrant a breaking change — the
entrypoint was moved from a separate binary to a unified one.

There is also the issue that other components, such as the Node Manager, were built
around the old `actyx` version and to keep everything working, we absolutely need to
keep that version number, question is where and how.

If we were going to do "self-publishing" only — i.e. as we were doing with Actions +
Actyx Releases — we could get away with using clap/structopt's features to distinguish
the versions of commands, but we are going to be publishing `ax` on crates.io, and that
warrants more care when publishing, since it means that users will be consuming this
by means of `cargo install ax`, and we need to be careful not to break their use cases.

Furthermore, the core "runnable" functionality — i.e. the `node` — was part of `actyx`
but is now part of `ax-core` which needs to be versioned as a library, as such, changes
to the library require bumps, which may not reflect on the `node`'s version.

## The Solution

To that end, we have decided to publish `ax-core` with its own version, starting at
`0.2.0` and inside it, the `node` will keep the previous `actyx` version and the CLI
will get a new scheme to ensure that users are able to know which version of `actyx`
the CLI has available.

The new scheme consists in ensuring that the MAJOR and MINOR versions of `ax` are the
same as the `node` version, PATCH will reflect other changes to the CLI.

Following this, we now have:

- `ax-core` at version `0.2.0`
- `node` at version `2.17`
- `ax` at version `2.17.0`

From the previous list, the new publishing "rules" become:

`ax-core`
- Features
  - will create a minor bump in `ax-core`
- Fixes
  - will create a patch bump in `ax-core`

`node`
- Features
  - will create a minor bump in `node`
  - will create a minor bump in `ax-core`
  - will create a minor bump in `ax`

- Fixes
  - will create a patch bump to `node` (?)
  - will create a patch bump to `ax-core`
  - will create a patch bump to `ax` (from the dependency version update)

`ax`
- Features & Fixes
  - will create a patch bump to `ax`

## Enforcement

Versions will keep being updated by the `release` tool, which will require updating
according to this change.

The `node` version will be kept in a Rust file to be managed by the `release` tool.

For example:

```rust
pub const NODE_VERSION: &str = "2.17.0"
```
