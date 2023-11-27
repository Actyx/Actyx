# Versioning Ax and friends
|  |  |
| --- | --- |
| date | 2023-11-24 |
| status | proposed |
| persons | @jmg-duarte, @rkuhn, @Kelerchian |

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
- `databank` - the old `node` component from `actyx`, which can be thought of as the inner Actyx
  "stack". We work off of the following definition:
  > A databank is a repository of information about one or more subjects, that is, a
  > database which is organized in a way that facilitates local or remote information
  > retrieval and is able to process many continual queries over a long period of time.

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
to the library require bumps, which may not reflect on the `databank`'s version.

## The Solution

To that end, we have decided to publish `ax-core` with its own version, starting at
`0.2.0` and inside it, the `databank` will keep the existing `actyx` version, the CLI
will move with the `databank` version.

Following this, we now have:

- `ax-core` at version `0.2.0`
- `databank` at version `2.17`
- `ax` at version `2.17.0`

From the previous list, the new publishing "rules" become:

`ax-core`
- Breaking changes
  - will create a major bump in `ax-core`
- Features
  - will create a minor bump in `ax-core`
- Fixes
  - will create a patch bump in `ax-core`

`databank`
- Breaking changes
  - will create a major bump in `databank`
  - will create a major bump in `ax-core`
  - will create a major bump in `ax`

- Features
  - will create a minor bump in `databank`
  - will create a minor bump in `ax-core`
  - will create a minor bump in `ax`

- Fixes
  - will create a patch bump to `ax-core`
  - will create a patch bump to `ax`

`ax`
 - Moves with `databank`

Publishing an `ax-core` release does not require a new `ax` release.

`ax` is to be released independently from `ax-core`, however, when publishing a new
release for `ax`, it will contain the latest `ax-core` release. Furthermore, version
bumps to `databank` will imply new `ax-core` and `ax` releases.

When developing, `ax` should always depend on the `ax-core` path and its most
up-to-date version.

## Enforcement

At the time of writing, the `databank` version is to be moved from `CARGO_PKG_VERSION`
to a separate, in-code, string dictating its current version.

For example:

```rust
pub const DATABANK_VERSION: &str = "2.17.0"
```

As it stands, the release
process is still being decided on, whether it is going to be done by hand or through
the `release` tool.

