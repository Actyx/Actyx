// paths relative to running nodejs process

const settings = {
  binaryPath: {
    ax: '../dist/bin/x64/ax',
    actyxosLinux: '../dist/bin/x64/actyxos-linux',
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
