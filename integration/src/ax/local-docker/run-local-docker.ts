import execa from 'execa'
import { platform } from 'os'

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
    ...opts,
    'actyx/os',
  ].join(' ')

// TODO: unit test
export const runOnLinux = mkRun(['--privileged --network=host'])
// TODO: unit test
export const runOnWinMac = mkRun()

export const runLocalDocker = async (actyxosDataPath: string): Promise<void> =>
  ['win32', 'darwin', 'linux'].includes(platform())
    ? execa
        .command((platform() === 'linux' ? runOnLinux : runOnWinMac)(actyxosDataPath))
        .then(() => undefined)
    : Promise.reject(`Cannot run Docker, platform ${platform()} is not supported!`)
