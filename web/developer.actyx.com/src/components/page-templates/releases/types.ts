export type Version = string
export type Hash = string
export type Product = string
export type Change = string
export type Release = {
  version: Version
  commit: Hash
  time: string
  changes: Change[]
}
export type ReleaseHistory = {
  ax?: Release[]
  actyx: Release[]
  cli: Release[]
  'node-manager': Release[]
}

export type Download = {
  platform: string
  ext: string
  files: {
    arch: string
    target: string
  }[]
}
