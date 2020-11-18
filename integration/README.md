# Integration testing

Test suites for combined Actyx products.

## How to use

Integration tests use primarily the artifacts built from the current commit, so you need to:

- Make ActyxOS and related binaries (`actyxos-linux` and `ax`) available by compiling them for the "current" platform as well as the platforms to be tested.
  The artifacts will be taken from `Cosmos/dist/bin/**` as well as DockerHub, use the `make all` command in the `Cosmos` folder
- run `npm install && npm run build` in `js/os-sdk` and `js/pond`

Then you can `npm install` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.

## Dev

| Scripts          | Description                                                        |
|------------------|--------------------------------------------------------------------|
| npm test         | Run test suites EC2 instances and local Docker                     |
| npm run lint:fix | Automatically fix lint issues                                      |
| npm run clean    | Remove the `temp` folder where test projects are cloned and built  |

| Environment variable               | Description                               |
|------------------------------------|-------------------------------------------|
| export AX_CI_HOSTS=your_hosts.yaml | Use a different selection of target hosts |

When developing test cases it is nicer to use a copy of `hosts.yaml` that only uses local nodes (like one with `install: linux` and as many as needed with `install: docker`).
This way the turnaround time is pretty short, allowing you to quickly iterate on only a specific test or suite.
In this case you may also want to disable the repeated preparation of the test projects.

**IMPORTANT PHILOSOPHY NOTE: Only add infrastructure (including configurability) when you actually need it, never add anything proactively!**

## Test Design

When creating tests, please follow the rules:

- Using nodes from `hosts.ts` (e.g. with `runOnEvery`) needs to consider these as shared resources:

    - no destructive actions like stopping all apps or changing `com.actyx.os` settings
    - the test must assume that other tests use the same nodes at the same time, so don’t assert “no apps running” or similar
    - do not change the committed `hosts.yaml` file unless you intend to add to the CI runs
    - do not add `type: local` nodes to the `hosts.yaml`

- Create per-suite nodes in a `beforeAll` hook, this way they will only be created if the suite actually runs and they will automatically be cleaned up afterwards.

- When referring to binaries, always go through the central `settings.ts` functions to allow consistent selection of versions.
  Add to `settings.ts` if facilities are missing.
