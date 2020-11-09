import execa from 'execa'

const mkRun = (opts: string[] = []) => (actyxosDataPath: string): string =>
  [
    'docker run',
    '--detach',
    '--name test-actyxos',
    '--rm',
    '-e AX_DEV_MODE=1',
    `-v ${actyxosDataPath}:/data`,
    '-p 4001:4001',
    '-p 4457:4457',
    '-p 127.0.0.1:4243:4243',
    '-p 127.0.0.1:4454:4454',
    '--privileged',
    ...opts,
    'actyx/os',
  ].join(' ')

// TODO: unit test
export const runOnLinux = mkRun(['--network=host'])
// TODO: unit test
export const runOnWinMac = mkRun()

const supportedPlatforms: NodeJS.Platform[] = ['win32', 'darwin', 'linux']
const getSpecificCmd = (x: NodeJS.Platform) => (x === 'linux' ? runOnLinux : runOnWinMac)

export const runLocalDocker = (platform: NodeJS.Platform, actyxosDataPath: string): Promise<void> =>
  supportedPlatforms.includes(platform)
    ? execa.command(getSpecificCmd(platform)(actyxosDataPath)).then(() => undefined)
    : Promise.reject(`Can not run Docker, platform ${platform} is not supported!`)
