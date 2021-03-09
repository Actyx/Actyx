import { CLI } from '../cli'
import { ApiClient } from '@actyx/os-sdk'
import { Arch, Host, OS } from '../../jest/types'

export type Target = {
  os: OS
  arch: Arch
  kind: TargetKind
  _private: {
    cleanup: () => Promise<void>
  }
}

export type SshAble = {
  host: string
  username: string
  privateKey: string
}

export type TargetKind =
  | ({ type: 'aws'; instance: string; privateAddress: string } & SshAble)
  | ({ type: 'ssh' } & SshAble)
  | { type: 'local' }
  | { type: 'test' }

export const printTarget = (t: Target): string => {
  const kind = t.kind
  switch (kind.type) {
    case 'aws': {
      return `AWS ${kind.instance} ${kind.host} ${t.os}/${t.arch}`
    }
    case 'ssh': {
      return `borrowed (SSH) ${kind.host} ${t.os}/${t.arch}`
    }
    case 'local': {
      return `borrowed (local) ${t.os}/${t.arch}`
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
}

export type ActyxOSNode = {
  name: string
  target: Target
  host: Host
  ax: CLI
  actyxOS: ApiClient
  _private: {
    shutdown: () => Promise<void>
    axBinaryPath: string
    axHost: string
    apiEvent: string
    apiConsole: string
    apiPond: string
  }
}

export type AwsKey = {
  keyName: string
  privateKey: string
  publicKeyPath: string
}
