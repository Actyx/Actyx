import execa from 'execa'
import { platform } from 'os'

const localDocker = () => {
  const cmdWinMac =
    'docker run --rm -v actyxos-data:/data --privileged -p 4001:4001 -p 4457:4457 actyx/os'
  const cmdLinux = 'docker run --rm -v actyxos-data:/data --privileged --network=host actyx/os'

  const isWinMac = platform() === 'win32' || platform() === 'darwin'
  const isLinux = platform() === 'linux'

  if (!isWinMac && !isLinux) {
    new Error(`Cannot run Docker, platform ${platform()} is not supported!`)
  }

  const cmd = isWinMac ? cmdWinMac : cmdLinux

  return {
    async setup() {
      try {
        execa.command(cmd)
        console.log('Running ActyxOS on Docker')
      } catch (error) {
        console.log('Make sure ActyxOS is not running locally when using integration tests!')
        console.error(error.stderr)
      }
    },
  }
}

export default localDocker()
