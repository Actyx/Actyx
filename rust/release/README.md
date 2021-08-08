# Cosmos Release

This calculates product version numbers and change logs.

## Design

The design goal is to Keep It Simple, Stupid.
Each command should do a very small thing, and do it well in a very predictable fashion.
There are as few options as possible; we can extend the program when, and only when, it becomes necessary.

## Usage

Compute the current version number for a product:

`cargo run -- version <product>`

This returns:
    - nothing if the version number hasn't change (i.e. no changes); or,
    - a single line with the new version number (e.g. 3.1.1).

Compute the current change log for a product:

`cargo run -- changes <product>`

This returns:
    - nothing if there is no changelog (i.e. no changes); or,
    - a single line for each change.

Get all past version numbers for a product:

`cargo run -- versions <product>`

This returns:
    - nothing if there are no past versions; or,
    - a single line for each past version (e.g. 3.1.1).

Compute the change log for a past version:

`cargo run -- changes <product> <version>`

This returns:
    - nothing if there is no changelog; or,
    - a single line for each change.

The `bin/persist-new-versions.sh` script shows how one could use this program to add possibly changed versions to the `versions` file.

## How it works

### Persistence of last released versions

The release versions for each Actyx product are stored in the `<COSMOS-ROOT>/versions` file with the newest versions below the older versions.
This program reads that file.

### Version and change log calculation

A change log is computed by looking at all commit messages since the last release and looking for specific lines containing information about what has changed.
These lines have the following shape: `<type>(<product>): <message>`.

The `type` may be any of:

- `break`: A breaking change
- `feat`: A new feature
- `build`: Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)
- `ci`: Changes to our CI configuration files and scripts (example scopes: Circle, BrowserStack, SauceLabs)
- `docs`: Documentation only changes
- `fix`: A bug fix
- `perf`: A code change that improves performance
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests or correcting existing tests

The `product` may be any of: `actyx`, `cli`, `node-manager`, `pond`, `rust-sdk`, `ts-sdk`, `csharp-sdk`.

The `message` may be any single-line string.

Then semantic version for each product is computed according to the found changes.
If a breaking change (e.g. `break(actyx): this breaks Actyx`) is found, the product's major version number is incremented.
If a new feature change (e.g. `feat(actyx): added Blob Store`) is found, the minor version is incremented.
For any other change, the patch number is incremented.

The program finds the underlying git repo using either `$GIT_DIR`, and if that is not set by walking up the directory tree to find a repo.
More details at https://docs.rs/git2/0.13.19/git2/struct.Repository.html#method.open_from_env.
