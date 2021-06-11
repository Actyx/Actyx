import execa from 'execa'
import { OS } from '../../jest/types'
import { Ssh } from './ssh'
import { TargetKind } from './types'

export type ExecuteFn = (
  file: string,
  params: string[],
  env?: { [_: string]: string },
) => execa.ExecaChildProcess<string>

export function mkExecute(os: OS, kind: TargetKind): ExecuteFn {
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      const ssh = Ssh.new(kind.host, kind.username, kind.privateKey)
      return (file: string, params: string[], env?: { [_: string]: string }) =>
        ssh.execFile(file, params, env)
    }
    case 'local':
    case 'test': {
      const shell = os === 'linux' ? '/bin/bash' : os === 'windows' ? 'powershell' : undefined
      return (script: string, params: string[], env) =>
        execa.command([script].concat(params).join(' '), { shell, env })
    }
  }
}
