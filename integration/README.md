# How to use this

Integration tests use primarily the artifacts built from the current commit, so you need to

- make ActyxOS binaries available by compiling them for all desired target platforms (will be taken from `dist/bin/*`)
- make the Actyx CLI binary available for the current host by compiling it (will be taken from `rt-master/target/release`)
- run `npm run build` in `js/os-sdk` and `js/pond`

Then you can `npm i` and `npm test` in this project. If you forgot to first build the other JS projects, you’ll have to remove `node_modules` and start over.
