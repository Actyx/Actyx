import execa from 'execa'
import fs from 'fs'
import path from 'path'
import { getFreePort } from './checkPort'
import { settings } from './settings'

export class Ssh {
  private commonOpts: string[]

  constructor(host: string, user: string, key: string) {
    const tempDir = settings().tempDir
    const keyFile = path.resolve(tempDir, 'keyFile.pem')
    const knownHosts = path.resolve(tempDir, 'known_hosts')
    try {
      fs.unlinkSync(keyFile)
    } catch (e) {
      // this is fine
    }
    fs.writeFileSync(keyFile, key, { mode: 0o400 })
    // it is important that user@host comes last, see scp
    this.commonOpts = [
      '-oConnectTimeout=5',
      `-oUserKnownHostsFile=${knownHosts}`,
      '-oStrictHostKeyChecking=off',
      `-i${keyFile}`,
      `${user}@${host}`,
    ]
  }

  exec(command: string): execa.ExecaChildProcess<string> {
    return execa('ssh', [...this.commonOpts, command])
  }

  async forwardPorts(...ports: number[]): Promise<[number[], execa.ExecaChildProcess<string>]> {
    const ret = ports.map(() => 0)
    const fwd = await Promise.all(
      ports.map(async (port, idx) => {
        const ours = await getFreePort()
        ret[idx] = ours
        return `-L${ours}:localhost:${port}`
      }),
    )
    return [ret, execa('ssh', [...fwd, '-nNf', ...this.commonOpts])]
  }

  async scp(mine: string, theirs: string): Promise<void> {
    const common = this.commonOpts.slice()
    const userHost = common.pop()
    const proc = execa('scp', [...common, mine, `${userHost}:${theirs}`])
    proc.stderr?.on('data', (chunk) => console.log('stderr', chunk))
    proc.stdout?.on('data', (chunk) => console.log('stdout', chunk))
    const result = await proc
    if (result.exitCode !== 0) {
      throw result
    }
  }
}
