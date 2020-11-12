# How to use this

Integration tests use primarily the artifacts built from the current commit, so you need to:

- make ActyxOS and related binaries (`actyxos-linux` and `ax`) available by compiling them for all desired target platforms (will be taken from `Cosmos/dist/bin/*`), use the `make` command in `Cosmos` folder for example: `make dist/bin/current/ax` and `make dist/bin/linux-x86_64/actyxos-linux`
- change permission for the binaries `chmod +x actyxos-linux` and `chmod +x ax`
- run `npm run build` in `js/os-sdk` and `js/pond`

Then you can `npm i` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.

## Dev

| Scripts                  | Description                                                       |
|--------------------------|-------------------------------------------------------------------|
| npm test                 | Run test suite using EC2 instanced and local Docker               |
| npm run test:localdocker | Run test suite using local Docker only                            |
| npm run lint:fix         | Automatically fix lint issues                                     |
| npm run clean:temp       | Remove the `temp` folder where test projects are cloned and built |

- To run only a single test file use for example: `npx tsc && npx jest -- ./dist/src/your-test.spec.js`

| Environment variable                                | Description              |
|-----------------------------------------------------|--------------------------|
| export AX_INTEGRATION_SKIP_SETUP_TEST_PROJECTS=true | Skip setup test projects |
