# How to use this

Integration tests use primarily the artifacts built from the current commit, so you need to

- make ActyxOS binaries (`actyxos-linux`) available by compiling them for all desired target platforms (will be taken from `Cosmos/dist/bin/*`), for instance `Cosmos/dist/bin/x64/actyxos-linux`
- make the Actyx CLI binary (`ax`) available for the current host by compiling it (from `Cosmos/rt-master/target/release`) and copy it to folder `Cosmos/dist/bin/`
- run `npm run build` in `js/os-sdk` and `js/pond`
- run `npm run lint:fix` to automatically fix lint issues

Then you can `npm i` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.

## Scripts

- use `npm run clean:temp` to remove the `temp` folder where test projects are cloned and builded

## Caveats

Currently on MacOS `actyxos-linux` binaries cannot be cross compiled for linux (`cargo build --release --target x86_64-unknown-linux-gnu --bin actyxos-linux`). To get these files, instead open Azure Pipelines website, and go to your build. At the top, you will see "Related" and "XX published". Click on "XX published", open actyxos-bin-x64 -> x66, you'll find the `actyxos-linux` binary there. This will only happen with master builds or with PR builds that modify rt-master or build.

## Dev

- To run only a single test file use for example `npm run tsc && jest -- ./dist/src/ax/your-test.spec.js`
