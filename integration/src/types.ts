import { Url } from 'url'

export type OS = 'win' | 'linux' | 'mac' | 'android'
export type Arch = 'armv7' | 'aarch64' | 'x86_64'
export type Host = 'docker' | 'process' | 'android'
export type Runtime = 'webview' | 'docker' | 'process'

export type Target = {
  os: OS
  arch: Arch
  /** base URL for deploying ActyxOS via Docker API */
  docker?: Url
  /** base path for deploying ActyxOS as host:dir */
  rsync?: string
  /** ADB connection string for deploying to Android */
  adb?: string
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
  os: OS
  arch: Arch
  runtimes: Runtime[]
  console: Url
  events: Url
}
