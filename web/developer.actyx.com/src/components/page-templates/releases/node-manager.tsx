import React from 'react'
import { Version, Change, Hash, Download } from './types'
import { Page as FileBasedPage } from './components/file-based-page'
import { NODE_MANAGER_DOWNLOADS_V1_1_5 } from './v1-legacy-downloads'
import semver from 'semver'

// $C gets replaced with the commit hash
// $V gets replaced with the version
const DOWNLOADS_V2_0_0: Download[] = [
  {
    platform: 'Linux',
    ext: '.deb',
    files: [
      {
        arch: 'amd64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-amd64.deb`,
      },
      //{
      //  arch: 'arm64',
      //  target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-arm64.tar.gz`,
      //},
      //{
      //  arch: 'armhf',
      //  target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-armhf.tar.gz`,
      //},
      //{
      //  arch: 'arm',
      //  target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-arm.tar.gz`,
      //},
    ],
  },
  {
    platform: 'Mac',
    ext: '.dmg',
    files: [
      {
        arch: 'universal',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/ActyxNodeManager-$V.dmg`,
      },
      //{
      //  arch: 'arm64',
      //  target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-macos-arm.zip`,
      //},
    ],
  },
  {
    platform: 'Windows (Installer)',
    ext: '.msi',
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-x64.msi`,
      },
    ],
  },
]

const downloads = (version: Version): Download[] => {
  if (semver.satisfies(version, '>=2.0.0')) {
    return DOWNLOADS_V2_0_0
  } else if (semver.satisfies(version, '1.1.5')) {
    return NODE_MANAGER_DOWNLOADS_V1_1_5
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
  return (
    <FileBasedPage
      {...data}
      product="node-manager"
      productDisplayName="Actyx Node Manager"
      downloads={downloads}
    />
  )
}

export default Page
