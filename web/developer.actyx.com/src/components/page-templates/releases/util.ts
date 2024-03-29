import { Version } from './types'
import semver from 'semver'

export const versionIsNewer = (v1: Version, v2: Version): boolean => {
  return semver.gt(v1, v2)
}

export const versionIsNewest = (v1: Version, others: Version[]): boolean => {
  let yes = true
  others.forEach((v) => {
    if (versionIsNewer(v, v1)) {
      yes = false
    }
  })
  return yes
}

export const newestVersion = (versions: Version[]): string => {
  let newest = versions[0]
  versions.forEach((v) => {
    if (versionIsNewer(v, newest)) {
      newest = v
    }
  })
  return newest
}
