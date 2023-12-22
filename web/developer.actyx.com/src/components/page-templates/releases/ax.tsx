import React from 'react'
import { Version, Change, Hash, Download } from './types'
import { Page as FileBasedPage } from './components/file-based-page'
import semver from 'semver'

// $C gets replaced with the commit hash
// $V gets replaced with the version
const DOWNLOADS_V2_0_0: Download[] = [
  {
    platform: 'Linux',
    ext: '.tar.gz',
    files: [
      {
        arch: 'amd64',
        target: `https://axartifacts.blob.core.windows.net/releases/ax-$V-linux-amd64.tar.gz`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/releases/ax-$V-linux-arm64.tar.gz`,
      },
      {
        arch: 'armhf',
        target: `https://axartifacts.blob.core.windows.net/releases/ax-$V-linux-armhf.tar.gz`,
      },
    ],
  },
  {
    platform: 'Mac',
    ext: '.zip',
    files: [
      {
        arch: 'intel',
        target: `https://axartifacts.blob.core.windows.net/releases/ax-$V-macos-intel.zip`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/releases/ax-$V-macos-arm.zip`,
      },
    ],
  },
  {
    platform: 'Windows',
    ext: '.msi',
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/releases/actyx-$V-x64.msi`,
      },
    ],
  },
  {
    platform: 'Android',
    ext: '.apk',
    files: [
      {
        arch: 'all',
        target: `https://axartifacts.blob.core.windows.net/releases/Actyx-$V.apk`,
      },
    ],
  },
]

const downloads = (version: Version): Download[] => {
  if (semver.satisfies(version, '>=2.18.0')) {
    return DOWNLOADS_V2_0_0
  } else {
    return []
  }
}

const Page: React.FC<{
  data: {
    version: Version
    commit: Hash
    changes: Change[]
    otherVersions: Version[]
  }
}> = ({ data }) => {
  console.log(data)
  return <FileBasedPage {...data} product="ax" productDisplayName="Ax" downloads={downloads} />
}

export default Page
