import React from 'react'
import { Product, Version, Change, Hash, Download } from './types'
import { CLI_DOWNLOADS_V1_1_5 } from './v1-legacy-downloads'
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
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-linux-amd64.tar.gz`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-linux-arm64.tar.gz`,
      },
      {
        arch: 'armhf',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-linux-armhf.tar.gz`,
      },
      {
        arch: 'arm',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-linux-arm.tar.gz`,
      },
    ],
  },
  {
    platform: 'Mac',
    ext: '.zip',
    files: [
      {
        arch: 'intel',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-macos-intel.zip`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-macos-arm.zip`,
      },
    ],
  },
  {
    platform: 'Windows',
    ext: '.zip',
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-windows-x64.zip`,
      },
    ],
  },
]
const downloads = (version: Version): Download[] => {
  if (semver.satisfies(version, '>=2.0.0')) {
    return DOWNLOADS_V2_0_0
  } else if (semver.satisfies(version, '1.1.5')) {
    return CLI_DOWNLOADS_V1_1_5
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
    product: Product
    productDisplayName: string
  }
}> = ({ data }) => {
  return (
    <FileBasedPage {...data} product="cli" productDisplayName="Actyx CLI" downloads={downloads} />
  )
}

export default Page
