# Integration testing

Test suites for combined Actyx products.

## How to use

Integration tests use primarily the artifacts built from the current commit, so you need to:

- Make Actyx and related binaries (`actyx-linux` and `ax`) available by compiling them for the "current" platform as well as the platforms to be tested.
  The artifacts will be taken from `Cosmos/dist/bin/**` as well as DockerHub, use the `make all` command in the `Cosmos` folder
- run `nvm use && npm install && npm run build` in `js/os-sdk` and `js/pond` (or run `make all-js`) in the `Cosmos` directory)

Then you can `nvm use`, `npm install`, and `npm test` in this project.

You can provide a dedicated git hash to test as an environment variable
`AX_GIT_HASH` or in the provided settings file.

## Dev

| Scripts          | Description                                              |
| ---------------- | -------------------------------------------------------- |
| npm test         | Run test suites EC2 instances and local Docker           |
| npm test:debug   | Run test suites using debugging support built into Node. |
| npm run lint:fix | Automatically fix lint issues                            |

Use bash script `./docker-build-tag.sh` to automatically build Docker container with latest git hash commit and appropriate tag.

### Environment variables

```sh
# Use different selection of target hosts
export AX_CI_HOSTS=your_hosts.yaml
# Use different git hash to test          |
export AX_GIT_HASH=18be8456847ec2a29ad53d30fa3ed9d533438cdf
```

When developing test cases it is faster to use a copy of `hosts.yaml` that only uses local nodes by setting `type: local` (like one with `install: linux` and as many as needed with `install: docker`), an example can be found at `my_hosts.yaml` which can be used with `export AX_CI_HOSTS=my_hosts.yaml`.
This way the turnaround time is pretty short, allowing you to quickly iterate on only a specific test or suite.

To debug in any Chromium-based browser use `npm test:debug` and open your browser and go to `chrome://inspect` and click on "Open Dedicated DevTools for Node", which will give you a list of available node instances you can connect to. Click on the address displayed in the terminal (usually something like localhost:9229) after running the above command, and you will be able to debug Jest using Chrome's DevTools.
The Chrome Developer Tools will be displayed, and a breakpoint will be set at the first line of the Jest CLI script. Click the "play" button in the upper right hand side of the screen to continue execution. When Jest executes the test that contains the debugger statement, execution will pause and you can examine the current scope and call stack.

**IMPORTANT PHILOSOPHY NOTE: Only add infrastructure (including configurations) when you need it, never add anything proactively!**

## Test Design

When creating tests, please follow the rules:

- Using nodes from `hosts.ts` (e.g. with `runOnEvery`) needs to consider these as shared resources:

  - no destructive actions like stopping all apps or changing `com.actyx` settings
  - the test must assume that other tests use the same nodes at the same time, so don’t assert “no apps running” or similar
  - do not change the committed `hosts.yaml` file unless you intend to add to the CI runs
  - do not add `type: local` nodes to the `hosts.yaml`

- Create per-suite nodes in a `beforeAll` hook using `createNode` (from `create.ts`), this way they will only be created if the suite actually runs and they will automatically be cleaned up afterward.

- When referring to binaries, always go through the central `settings.ts` functions to allow for a consistent selection of versions.
  Add to `settings.ts` if facilities are missing.

## Notes for local development

If are developing against `type: local` nodes and you terminate Jest by using CTRL+C repeatedly (hitting CTRL+C once should tear down everything) when it is running, make sure to stop the related Docker containers if are still running.

If you have configured `gitHash: null` on your local branch, only binaries will be built automatically.

To write test case locally which are using a Docker test node (`createTestNodeDockerLocal`) you need to build the Docker image manually and load it in the Docker daemon, this operation should be performed after each commit. To do what you can run `./docker-build-tag.sh` or can be done manually.

## Hacks

If you are writing test cases for the event-service for a single local node only and you want to bypass the custom Jest setup to save some time, you can add in `package.json` the following script:
`"test:es": "npm run build && TZ=UTC jest --config=jest.config.event-service.js"`.
Change also `baseURL` in `httpClient.ts`. Now you should run the event service locally and make sure to do not commit changes to `package.json` file.
