import { MyGlobal } from '../../jest/setup'
import { Arch, Settings } from '../../jest/types'

const setup = (<MyGlobal>global).axNodeSetup

export const settings = (): Settings => setup.settings

export const currentAxBinary = '../dist/bin/current/ax'
export const currentActyxOsBinary = '../dist/bin/current/actyxos-linux'
export const actyxOsLinuxBinary = (arch: Arch): string => `../dist/bin/linux-${arch}/actyxos-linux`
export const actyxOsDockerImage = (arch: Arch, version: string): string =>
  `actyx/cosmos:actyxos-${arch}-${version}`
