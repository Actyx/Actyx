import execa from 'execa'
import { OS } from '../../jest/types'
import { Ssh } from './ssh'
import { TargetKind } from './types'

type ExecuteFn = (
  script: string,
) => Promise<{
  exitCode: number
  stdOut: string
  stdErr: string
}>

export function mkExecute(os: OS, kind: TargetKind): ExecuteFn {
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      const mkSsh = () => Ssh.new(kind.host, kind.username, kind.privateKey)
      return async (script: string) => {
        const res = await mkSsh().exec(script)
        return {
          exitCode: res.exitCode,
          stdOut: res.stdout,
          stdErr: res.stderr,
        }
      }
    }
    case 'local': {
      const shell = os === 'linux' ? '/bin/bash' : os === 'windows' ? 'powershell' : undefined
      return async (script: string) => {
        const result = await execa.command(script, { shell })
        return {
          exitCode: result.exitCode,
          stdOut: result.stdout,
          stdErr: result.stderr,
        }
      }
    }
    case 'test': {
      return () => Promise.resolve({ exitCode: 0, stdOut: '', stdErr: '' })
    }
  }
}
