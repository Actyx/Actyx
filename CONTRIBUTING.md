# Contributing to Actyx

Welcome! This document outlines the various ways you can contribute to the project.
Take a moment to read through the guidelines before you get started.

## Questions, Suggestions, and Bug Reports

We value your input!

<!-- (what are we doing there when after we open-sourced Actyx repo)? -->
For questions and light discussions, we are generally available in:
- Discord <!-- (TODO: discord link)-->
- Google groups <!--(TODO: google groups link)-->

However, if you have well-planned suggestions (e.g. features, optimizations, refactors) or discovered bugs, use our github issue tracker (TODO: link). Before posting an issue, make sure you've searched for a duplicate. The following are guidelines for creating issues:

### Bug Reports

A bug report must include:

- Bug summary
- Expected and Actual Behavior
- Minimum Code or Steps to Reproduce
- Environment Details (Actyx version, OS, Device Details, any other relevant information)

### Suggestions

A suggestion must include:

- Suggestion Summary
- Description
- Use case
- If a suggestion involves a change to CI scripts, "Justification" section must be included, detailing the clear benefits and risks for the changes.

## Contributing Code

### Quick Start

1. **Clone and Fork the Repository**: Start by forking the repository. Clone your fork to your local machine using the following command:

    ```
    TODO: 
    ```

2. **Ensure Tests Pass**: Before making any changes, make sure that the existing tests are passing. Run the test suite with:

    ```
    TODO: 
    ```

3. **Submit a Pull Request (PR)**: Create a fork of Actyx. Commit your changes by following our commit message convention (TODO: link). Open a pull request to the `main` branch of the main repository.

### Commit Message

We intend to keep a clear commit history to make it easy to track the evolution of a codebase.

<!-- Do we want to use conventional commits? -->
This repository uses [Conventional Commit](https://www.conventionalcommits.org/) to keep commit messages consistent and contextual.

In addition, commit message must be concise, meaningful, and readable, and must use an imperative mood.

### Pull Requests

All changes from a fork must go through a Pull Request before being merged to the main repository.
This enables peer reviews to ensure that the code quality are upheld.
To maintain an efficient review process, pull requests must:

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
