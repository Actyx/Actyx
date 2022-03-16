import { execaCommand, ExecaChildProcess } from 'execa'
import { OS } from '../jest/types'
import { Ssh } from './ssh'
import { TargetKind } from './types'

export type ExecuteFn = (
  file: string,
  params: string[],
  env?: { [_: string]: string },
) => { process: ExecaChildProcess<string>; ssh?: Ssh }

export function mkExecute(os: OS, kind: TargetKind): ExecuteFn {
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      const ssh = Ssh.new(kind.host, kind.username, kind.privateKey)
      return (file: string, params: string[], env?: { [_: string]: string }) => ({
        process: ssh.execFile(file, params, env),
        ssh,
      })
    }
    case 'local':
    case 'test': {
      const shell =
        os === 'linux' || os === 'macos' ? '/bin/bash' : os === 'windows' ? 'powershell' : undefined
      return (script: string, params: string[], env) => ({
        process: execaCommand([script].concat(params).join(' '), { shell, env }),
      })
    }
  }
}
