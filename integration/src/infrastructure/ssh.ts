import execa from 'execa'
import fs from 'fs'
import { removeSync } from 'fs-extra'
import path from 'path'
import { getFreePort } from './checkPort'
import { netString } from './mkProcessLogger'
import { settings } from './settings'

export class Ssh {
  private commonOpts: string[]

  constructor(host: string, user: string, key: string) {
    const tempDir = settings().tempDir
    const keyFile = path.resolve(tempDir, 'keyFile.pem')
    const knownHosts = path.resolve(tempDir, 'known_hosts')
    removeSync(keyFile)
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
    const target = common.pop() + ':' + theirs
    const proc = execa('scp', [...common, mine, target])
    proc.stderr?.on('data', (chunk) =>
      console.log('scp %s -> %s [stderr]', mine, target, netString(chunk)),
    )
    proc.stdout?.on('data', (chunk) =>
      console.log('scp %s -> %s [stdout]', mine, target, netString(chunk)),
    )
    const result = await proc
    if (result.exitCode !== 0) {
      throw result
    }
  }
}
