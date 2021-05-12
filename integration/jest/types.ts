import * as t from 'io-ts'
import { fromNullable } from 'io-ts-types'

export const OS = t.keyof({ linux: 0, windows: 0, macos: 0, android: 0 })
export type OS = t.TypeOf<typeof OS>
export const currentOS = (): OS => {
  switch (process.platform) {
    case 'android':
      return 'android'
    case 'win32':
      return 'windows'
    case 'darwin':
      return 'macos'
    case 'linux':
      return 'linux'
    default:
      throw new Error(`cannot run on platform '${process.platform}'`)
  }
}

export const Arch = t.keyof({ x86_64: 0, aarch64: 0, armv7: 0, arm: 0 })
export type Arch = t.TypeOf<typeof Arch>
export const currentArch = (): Arch => {
  switch (process.arch) {
    case 'x64':
      return 'x86_64'
    case 'arm':
      return 'armv7' // hmm, how to detect absence of hardware float?
    case 'arm64':
      return 'aarch64'
    default:
      throw new Error(`cannot run on architecture '${process.arch}'`)
  }
}

export type Host = 'docker' | 'process' | 'android'

const createEC2 = t.type({
  type: t.literal('create-aws-ec2'),
  ami: t.string,
  instance: t.string,
  user: t.string,
  armv7: fromNullable(t.boolean)(false),
})
export type CreateEC2 = t.TypeOf<typeof createEC2>

const useLocal = t.type({
  type: t.literal('local'),
})

const useSsh = t.type({
  type: t.literal('ssh'),
  host: t.string,
  user: t.union([t.string, t.undefined]),
  privateKeyPath: t.union([t.string, t.undefined]),
  arch: Arch,
  os: OS,
})
export type UseSsh = t.TypeOf<typeof useSsh>

const prepare = t.union([createEC2, useLocal, useSsh])

const install = t.union([
  // deploy binaries or images
  t.type({
    type: t.keyof({ windows: 0, linux: 0, docker: 0, android: 0 }),
  }),
  // just use a running Actyx node
  t.type({
    type: t.literal('just-use-a-running-actyx-node'),
    host: fromNullable(t.string)('localhost'),
    console: t.number,
    services: t.number,
    pond: t.number,
  }),
])

const host = t.type({ name: t.string, install, prepare })
export type HostConfig = t.TypeOf<typeof host>

const settings = t.type({
  tempDir: t.string,
  keepNodesRunning: t.boolean,
  gitHash: t.union([t.string, t.null]),
  // Rather than writing all logs to individual files, dump everything on stdout
  logToStdout: t.boolean,
})
export type Settings = t.TypeOf<typeof settings>

export const Config = t.type({ hosts: t.array(host), settings })
export type Config = t.TypeOf<typeof Config>
