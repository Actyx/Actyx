# Contributing to Actyx

Welcome to Actyx!
This document outlines the various ways you can contribute to the project.
Take a moment to read through the guidelines before you get started.

## Questions, Suggestions, and Bug Reports

We value your input!

For questions and light discussions, we are generally available in:
- [Discord](https://discord.gg/4RZpTqmPgC)
- [Github Discussion](https://github.com/Actyx/Actyx/discussions)

If you have suggestions or encountered bugs, submit them to our [github issue tracker](https://github.com/Actyx/Actyx/issues).
Make sure you have searched for a duplicate.

## Contributing Code

### Commit Message

We intend to keep it easy to track the evolution of a codebase.
One rule to follow: A commit message must be a descriptive and concise one-line summary, followed by as much body text as necessary.

### Pull Requests

To maintain an efficient review process, pull requests should:

1. Reference an issue: Use GitHub's referencing syntax such as "Resolves #123" to automatically link the PR to the relevant issue to make sure the reviewers understand the context of the change.
2. Pass an automated check: An automated is set to ensure code quality and stability. This includes past unit tests and linters. Make sure all checks are passed.
3. Include Unit Tests (For New Features and Modified Specifications): Unit tests validate new functionalities and prevent regressions from future changes.

<!--
When do we bump? 
Surely not on PR since it doesn't scale? 
Do we need a new process for this?
-->
### Changes and versions

Actyx uses [SemVer](https://semver.org/) to manage and communicate code changes to users. 
A quick guide about SemVer:

- SemVer consists of three components: `MAJOR.MINOR.PATCH` (e.g. `2.16.1`)
- `MAJOR` is incremented when a breaking change is introduced, requiring users to update their code and configurations.
- `MINOR` is incremented when a new feature is added in a manner that is backward-compatible and not disrupting existing usage.
- `PATCH` is incremented when a backward-compatible bug fixes or improvements are introduced.

<!--
QUESTION: ay=ny thoughts about sub-folder CONTRIBUTOR.MD?

Scrapped because each projects needs its own `getting started`

## Getting Started With Actyx For Advanced Users (TODO: content)

## Code Structure (TODO: content)

## Interesting Entry Points (TODO: content)
-->
