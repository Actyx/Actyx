import execa from 'execa'
import { Observable } from 'rxjs'
import { OS } from '../../jest/types'
import { netString } from './mkProcessLogger'
import { Ssh } from './ssh'
import { Target, TargetKind } from './types'
import { mkCmd } from './windows'

export type ExecuteFn = (
  file: string,
  params: string[],
  env?: { [_: string]: string },
) => execa.ExecaChildProcess<string>

export function mkExecute(os: OS, kind: TargetKind): ExecuteFn {
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      const mkSsh = () => Ssh.new(kind.host, kind.username, kind.privateKey)
      return (file: string, params: string[], env?: { [_: string]: string }) =>
        mkSsh().execFile(file, params, env)
    }
    case 'local':
    case 'test': {
      const shell = os === 'linux' ? '/bin/bash' : os === 'windows' ? 'powershell' : undefined
      return (script: string, params: string[], env) =>
        execa.command([script].concat(params).join(' '), { shell, env })
    }
  }
}
function toObservable(proc: execa.ExecaChildProcess<string>): Observable<string> {
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
    proc.stdout?.on('data', (l) => emitLog('stdout', l))
    proc.stderr?.on('data', (l) => emitLog('stderr', l))

    proc.on('error', (err) => {
      flush()
      observer.error(err)
    })
    proc.on('exit', () => {
      flush()
      observer.complete()
    })
    return () => proc.kill('SIGKILL')
  })
}

export async function startEphemeralProcess(
  target: Target,
  defaultBinaryLocation: string,
  params: string[],
): Promise<[execa.ExecaChildProcess<string>]> {
  switch (target.os) {
    case 'android':
    case 'macos': {
      return Promise.reject(`Starting ephemeral nodes not supported on ${target.os}`)
    }
    case 'linux': {
      const ACTYX_PATH = (await target.execute('mktemp -d', [])).stdout.trim()
      const proc = (target.executeInContainer || target.execute)(defaultBinaryLocation, params, {
        ACTYX_PATH,
      })
      return [proc]
    }
    case 'windows': {
      const workingDir = (
        await target.execute(
          String.raw`$tempFolderPath = Join-Path $Env:Temp $(New-Guid)
        New-Item -Type Directory -Path $tempFolderPath | Out-Null
        $out = Join-Path $tempFolderPath id
        $out`,
          [],
        )
      ).stdout
      const cmd = mkCmd(defaultBinaryLocation, ['--working-dir', workingDir].concat(params))
      const proc = target.execute(cmd, [])
      return [proc]
    }
  }
}

export async function startEphemeralNode(
  target: Target,
  defaultBinaryLocation: string,
  params: string[],
): Promise<Observable<string>> {
  const proc = await startEphemeralProcess(target, defaultBinaryLocation, params)
  return toObservable(proc[0])
}
