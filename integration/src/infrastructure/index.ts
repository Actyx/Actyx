import execa, { ExecaChildProcess } from 'execa'
import { Observable } from 'rxjs'
import { OS } from '../../jest/types'
import { netString } from './mkProcessLogger'
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

export function toObservable(proc: Promise<[ExecaChildProcess]>): Observable<string> {
  return new Observable((observer) => {
    const lines = { stdout: '', stderr: '' }
    const emitLog = (where: keyof typeof lines, s: Buffer | string) => {
      const l = (lines[where] + netString(s)).split('\n')
      lines[where] = l.pop() || ''
      for (const line of l) {
        observer.next(`(${where}): ${line}`)
      }
    }
    const flush = () => {
      if (lines.stdout !== '') {
        emitLog('stdout', '\n')
      }
      if (lines.stderr !== '') {
        emitLog('stderr', '\n')
      }
    }
    proc.then(([p]) => {
      p.stdout?.on('data', (l) => emitLog('stdout', l))
      p.stderr?.on('data', (l) => emitLog('stderr', l))

      p.on('error', (err) => {
        flush()
        observer.error(err)
      })
      p.on('exit', () => {
        flush()
        observer.complete()
      })
    })
    return () => proc.then(([p]) => p.kill('SIGKILL'))
  })
}
