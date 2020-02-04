[![Build Status](https://travis-ci.com/Actyx/Internal-Ada.svg?token=WyU3d46Z7fFVsHpb9uuE&branch=master)](https://travis-ci.com/Actyx/Internal-Ada)
[![code style: prettier](https://img.shields.io/badge/code_style-prettier-ff69b4.svg?style=flat-square)](https://github.com/prettier/prettier)

# API

[Fishes](Fishes.md)

If you want to change some package and see changes at another where it's used as a dependency use [link](https://docs.npmjs.com/cli/link)

# Event storage

Details about how events are stored can be found in the [ipfs store](IpfsStore.md) documentation. Details about
troubleshooting ipfs based storage and dissemination can be found in the [troubleshooting guide](Troubleshooting.md).

# npm link

npm link allows linking a local version of ada to a project using ada such as iris for quick development of ada.

## Basic workflow

* `npm link` in ada
* `npm link actyx-ada` in project depending on ada (e.g. iris)
  this creates a symbolic link from the node_modules of iris to the checked out ada version
* run `npm run build:commonjs -- --watch` in ada to make sure the module is being continuously updated. The produced js code in `./lib` is used for both the browser and nodejs code (test or headless pond).
* exclude actyx-ada from the libraries that go into the prebuilt iris dependencies in `webpack.dll.config.js`.
* to run iris ui, run `npm run start:dev` in iris. Any change in ada will trigger a recompile of ada, which will be picked up and immediately applied
* any time a change to package dependencies is made, `npm start` or `npm build:dll && npm start:dev` need to be executed in iris
* when finished, do `npm unlink actyx-ada` in iris, run `npm install` or any command that triggers `npm install`, such as `npm start`.

Do not forget to unlink when done. Do not forget to run both packages in `watch` mode. When you make changes to a dependency you have to refresh manually package that is dependent on it, because we don't have any notifications configured for changes in package dependencies.

## Troubleshooting

If you get strange compile errors when linking e.g. ada to iris, make sure that the library versions in ada and iris match. E.g. there have been problems when the version of rxjs in ada was not the same as the one in iris.

## Testing on your device

The configuration is stored in IPFS, which is mirrored in the [Internal-Deployment-Config](https://github.com/Actyx/Internal-Deployment-Config) repository. The [Bootstrap Bag](https://github.com/Actyx/Internal-Deployment-Config/tree/master/deviceConfig/api/bootstrap) contains a JSON file for each device, named after its serial number (you can get it from Android tablets via `adb get-serialno`). The `factoryConfig` key in that file points to a directory inside the [swarms](https://github.com/Actyx/Internal-Deployment-Config/tree/master/swarms) directory, which contains again one file per device. Inside that file, the `root` key points to the content of the root app (Iris or whatever).

It is recommended that you create an IPNS name for yourself and possibly for each of your device/application combinations©∂. You can create the key on your own workstation, and point it to the IPFS hash that contains the version you want to deploy. That way you can change application versions without having to modify the `Internal-Deployment-Config` directory.

## Performance testing harness

In `src/performanceHarness` there is a performance testing harness. You can run it locally
like this:

`npm run injectMachineEvents --duration P1D --period PDT1000S --start 2018-01-01T00:00:00Z`

or in conjunction with local RDS:
`DEBUG=* npm run injectMachineEvents --duration P1D --period PDT1000S --start 2018-01-01T00:00:00Z --remote ws://local:local@localhost:5000/api/v1/events`

The sqlite files will be created in local directory.

The harness runner (`src/performanceHarness/runner.ts`) has built-in facility to perform heap dumps
that can be later read in the Chrome browser (Profile -> Memory -> Profiles -> Open...)

In order to load the library, small initial heapdump is taken on start. It uses the default
location of `/tmp`. In case it breaks for you, update the directory in the following line of
`runner.ts`:

`heapdump.writeSnapshot('/tmp/' + Date.now() + '.heapsnapshot')`

In order to take a heapdump while the program is running, you can simply `kill -USR2 <pid>`

The program is stopped for the time it is necessary to collect the dump and then continues.

Before the heap is dumped, GC is executed:

```
[17726:0x314fa60]     2290 ms: Mark-sweep 48.1 (81.0) -> 34.0 (79.5) MB, 23.5 / 0.0 ms  heap profiler GC in old space requested
[17726:0x314fa60]     2316 ms: Mark-sweep 34.0 (79.5) -> 33.7 (75.0) MB, 26.5 / 0.0 ms  heap profiler GC in old space requested
```

More details - [heapdump](https://github.com/bnoordhuis/node-heapdump)

You can enable the tracing of gc using the `--trace_gc` node flag (please note in ts_node the node flags use underscores instead of dashes!)
by altering the relevant line in `package.json` to this
`"injectMachineEvents": "ts-node --trace_gc --max_old_space_size=4096 src/performanceHarness/runner.ts"`

There are also `--trace_gc_verbose` and `--trace_fragmentation` options - see [node gc flags](https://gist.github.com/listochkin/10973974)

For fuller output:
`"injectMachineEvents": "ts-node --trace_gc --trace_gc_verbose --trace_fragmentation --max_old_space_size=4096 src/performanceHarness/runner.ts"`

Read more here on [the diagnostic output formats and node gc internals](https://www.slideshare.net/NodejsFoundation/are-your-v8-garbage-collection-logs-speaking-to-youjoyee-cheung-alibaba-cloudalibaba-group)
