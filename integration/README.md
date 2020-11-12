# How to use this

Integration tests use primarily the artifacts built from the current commit, so you need to:

- make ActyxOS and related binaries (`actyxos-linux` and `ax`) available by compiling them for all desired target platforms (will be taken from `Cosmos/dist/bin/*`), use the `make` command in `Cosmos` folder for example: `make dist/bin/current/ax` and `make dist/bin/linux-x86_64/actyxos-linux`
- change permission for the binaries `chmod +x actyxos-linux` and `chmod +x ax`
- run `npm run build` in `js/os-sdk` and `js/pond`

Then you can `npm i` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.

## Dev

| Scripts            | Description                                                       |
|--------------------|-------------------------------------------------------------------|
| npm test           | run all complete test suite                                       |
| npm run lint:fix   | automatically fix lint issues                                     |
| npm run clean:temp | remove the `temp` folder where test projects are cloned and built |

- To run only a single test file use for example: `npx tsc && npx jest -- ./dist/src/ax/your-test.spec.js`

| Environment variable                                | Description                                                       |
|-----------------------------------------------------|-------------------------------------------------------------------|
| export AX_INTEGRATION_SKIP_SETUP_TEST_PROJECTS=true | skip setup test projects                                          |
| export AX_INTEGRATION_SKIP_EC2=true                 | skip testing using EC instances and use instead only local Docker |
