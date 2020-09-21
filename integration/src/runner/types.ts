import { CLI } from '../ax'

export type OS = 'win' | 'linux' | 'mac' | 'android'
export type Arch = 'armv7' | 'aarch64' | 'x86_64'
export type Host = 'docker' | 'process' | 'android'
export type Runtime = 'webview' | 'docker' | 'process'

export type Target = {
  os: OS
  arch: Arch
  kind: TargetKind
  shutdown: () => void
}

export type SshAble = {
  host: string
  username: string
  privateKey: string
}

export type TargetKind =
  | ({ type: 'aws'; instance: string } & SshAble)
  | ({ type: 'borrowed' } & SshAble)
  | 'test'

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
  shutdown: () => void
}

export type AwsKey = {
  keyName: string
  privateKey: string
}
