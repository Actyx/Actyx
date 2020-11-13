# Integration testing

Test suites for combined Actyx products.

## How to use

Integration tests use primarily the artifacts built from the current commit, so you need to:

- make ActyxOS and related binaries (`actyxos-linux` and `ax`) available by compiling them for all desired target platforms (will be taken from `Cosmos/dist/bin/*`), use the `make` command in `Cosmos` folder for example: `make dist/bin/current/ax` and `make dist/bin/linux-x86_64/actyxos-linux`
- change permission for the binaries `chmod +x actyxos-linux` and `chmod +x ax` [workaroun for now]
- run `npm run build` in `js/os-sdk` and `js/pond`

Then you can `npm install` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.

## Dev

| Scripts                          | Description                                                        |
|----------------------------------|--------------------------------------------------------------------|
| npm test                         | Run test suites EC2 instances and local Docker                     |
| npm run test:localdocker         | Run test suite local Docker only                                   |
| npm run test:localdocker:nosetup | Run test suite using local Docker only and skip test project setup |
| npm run lint:fix                 | Automatically fix lint issues                                      |
| npm run clean:temp               | Remove the `temp` folder where test projects are cloned and built  |

| Environment variable                                | Description              |
|-----------------------------------------------------|--------------------------|
| export AX_INTEGRATION_SKIP_SETUP_TEST_PROJECTS=true | Skip setup test projects |

- to run only a single test file use for example: `npx tsc && npx jest --config=jest.local-docker.config.js -- ./dist/src/yourtest.spec.js`
- common settings are included in `settings.ts`

## Local Docker test suite

The local Docker test suite, usable with `npm run test:localdocker` will test the `ax` cli against a *single* node ActyxOS on Docker published on Docker Hub.
Tests run serially and each test should be executed in a "clean" test environment.
To reset the test environment for each test file the developer has to call the following utility function:

```typescript
  beforeAll(async () => {
    await resetTestEviroment()
  })
  afterAll(async () => {
    await resetTestEviroment()
  })
```
