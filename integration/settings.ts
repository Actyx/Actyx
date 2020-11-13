// paths relative to running nodejs process

const settings = {
  binaryPath: {
    ax: '../dist/bin/linux-x86_64/ax', //TODO: make it more flexible
    actyxosLinux: '../dist/bin/linux-x86_64/actyxos-linux',
  },
  localDocker: {
    containerName: 'test-actyxos',
    pull: 'actyx/os:1.0.0',
  },
  testProjects: {
    tempDir: 'temp',
  },
}

export default settings
