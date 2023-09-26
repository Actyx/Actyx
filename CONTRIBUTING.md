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
3. Include unit tests (for new features and modified specifications): Unit tests validate new functionalities and prevent regressions from future changes.


## Release Process

Actyx's versions are released by merging release commits, these are automatically generated when a commit's body contains the following shape: `<type>(<product>): <message>`.
As a developer, you should not need to run the release tool, unless you're checking for your change manually.

For a detailed view of the release tool, check it's [README](./rust/release/README.md).

### Changes and versions

Actyx uses [SemVer](https://semver.org/) to manage and communicate code changes to users.
A quick guide about SemVer:

- SemVer consists of three components: `MAJOR.MINOR.PATCH` (e.g. `2.16.1`)
- `MAJOR` is incremented when a breaking change is introduced, requiring users to update their code and configurations.
- `MINOR` is incremented when a new feature is added in a manner that is backward-compatible and not disrupting existing usage.
- `PATCH` is incremented when a backward-compatible bug fixes or improvements are introduced.
