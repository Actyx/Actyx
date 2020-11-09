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
    '--privileged',
    ...opts,
    'actyx/os',
  ].join(' ')

// TODO: unit test
export const runOnLinux = mkRun(['--network=host'])
// TODO: unit test
export const runOnWinMac = mkRun()

const tap = <T>(cb: (x: T) => void) => (x: T): T => {
  cb(x)
  return x
}

export const runLocalDocker = async (actyxosDataPath: string): Promise<void> =>
  ['win32', 'darwin', 'linux'].includes(platform())
    ? execa
        .command(
          tap(console.log)((platform() === 'linux' ? runOnLinux : runOnWinMac)(actyxosDataPath)),
        )
        .then(() => undefined)
    : Promise.reject(`Cannot run Docker, platform ${platform()} is not supported!`)
