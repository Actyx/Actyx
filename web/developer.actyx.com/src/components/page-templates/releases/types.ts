export type Version = string
export type Hash = string
export type Product = string
export type Change = string
export type Release = {
  version: Version
  commit: Hash
  changes: Change[]
}
export type ReleaseHistory = {
  actyx: Release[]
  cli: Release[]
  'node-manager': Release[]
  pond: Release[]
}

export type Download = {
  platform: string
  ext: string
  files: {
    arch: string
    target: string
  }[]
}
