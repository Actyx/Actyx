import { CLI } from '../ax'
import { ApiClient } from '@actyx/os-sdk'

export type OS = 'win' | 'linux' | 'mac' | 'android'
export type Arch = 'armv7' | 'aarch64' | 'x86_64'
export type Host = 'docker' | 'process' | 'android'
export type Runtime = 'webview' | 'docker' | 'process'

export type Target = {
  os: OS
  arch: Arch
  kind: TargetKind
  _private: {
    shutdown: () => Promise<void>
  }
}

export type SshAble = {
  host: string
  username: string
  privateKey: string
}

export type TargetKind =
  | ({ type: 'aws'; instance: string; privateAddress: string } & SshAble)
  | ({ type: 'borrowed' } & SshAble)
  | { type: 'test' }

export const printTarget = (t: Target): string => {
  const kind = t.kind
  switch (kind.type) {
    case 'aws': {
      return `AWS ${kind.instance} ${kind.host} ${t.os}/${t.arch}`
    }
    case 'borrowed': {
      return `borrowed ${kind.host} ${t.os}/${t.arch}`
    }
    case 'test': {
      return `test ${t.os}/${t.arch}`
    }
  }
}

export type NodeSelection = {
  os?: OS
  arch?: Arch
  host?: Host
  runtime?: Runtime
}

export type ActyxOSNode = {
  name: string
  target: Target
  host: Host
  runtimes: Runtime[]
  ax: CLI
  actyxOS: ApiClient
  _private: {
    shutdown: () => Promise<void>
    axBinary: string
    axHost: string
    apiEvent: string
    apiConsole: string
    apiPond: string
  }
}

export type AwsKey = {
  keyName: string
  privateKey: string
}
