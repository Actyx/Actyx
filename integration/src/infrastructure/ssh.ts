import execa, { ExecaChildProcess } from 'execa'
import fs from 'fs'
import { removeSync } from 'fs-extra'
import path from 'path'
import { getFreePort } from './checkPort'
import { netString } from './mkProcessLogger'
import { settings } from './settings'
import { portBound } from './util'

export class Ssh {
  private commonOpts: string[]

  private constructor(opts: string[]) {
    this.commonOpts = opts
  }
  public static new(host: string, user?: string, key?: string): Ssh {
    if (user !== undefined && key !== undefined) {
      return Ssh.newWithKey(host, user, key)
    } else {
      return Ssh.newWithHost(host)
    }
  }

  static newWithKey(host: string, user: string, key: string): Ssh {
    const tempDir = settings().tempDir
    const keyFile = path.resolve(tempDir, 'keyFile.pem')
    const knownHosts = path.resolve(tempDir, 'known_hosts')
    removeSync(keyFile)
    fs.writeFileSync(keyFile, key, { mode: 0o400 })
    // it is important that user@host comes last, see scp
    return new Ssh([
      '-oConnectTimeout=5',
      `-oUserKnownHostsFile=${knownHosts}`,
      '-oStrictHostKeyChecking=off',
      `-i${keyFile}`,
      `${user}@${host}`,
    ])
  }

  static newWithHost(host: string): Ssh {
    return new Ssh(['-oConnectTimeout=5', host])
  }

  exec(command: string): execa.ExecaChildProcess<string> {
    return execa('ssh', [...this.commonOpts, command])
  }

  execFile(
    file: string,
    params: string[],
    env?: { [_: string]: string },
  ): execa.ExecaChildProcess<string> {
    const e = Object.entries(env || {}).reduce(
      (acc, [k, v]) => acc.concat([`${k}=${v}`]),
      [] as string[],
    )

    return execa('ssh', [...this.commonOpts, ...e, file, ...params])
  }

  async forwardPorts(...ports: number[]): Promise<[number[], ExecaChildProcess]> {
    const ret = ports.map(() => 0)
    const fwd = await Promise.all(
      ports.map(async (port, idx) => {
        const ours = await getFreePort()
        ret[idx] = ours
        // Windows port forwarding doesn't work with `localhost`
        return `-L${ours}:127.0.0.1:${port}`
      }),
    )
    console.log(`forwarding ports: ${JSON.stringify(fwd)}`)
    // unfortunately there seems to be no portable way to await successful port forwarding within ssh
    const proc = execa('ssh', [...fwd, '-nNf', ...this.commonOpts])
    // so try to connect
    for (const port of ret) {
      await portBound(port)
    }
    return [ret, proc]
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
