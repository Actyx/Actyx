import React from 'react'
import { Product, Version, Change, Hash, Download } from './types'
import { Page as FileBasedPage } from './components/file-based-page'

// $C gets replaced with the commit hash
// $V gets replaced with the version
const DOWNLOADS: Download[] = [
  {
    platform: "Linux",
    ext: ".tar.gz",
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
    ]
  },
  {

    platform: "Mac",
    ext: ".zip",
    files: [
      {
        arch: 'intel',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-macos-intel.zip`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-macos-arm.zip`,
      },
    ]
  },
  {
    platform: "Windows (Installer)", ext: ".zip",
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-cli-$V-installer-windows-x64.zip`,
      },
    ]
  },
]

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
    <FileBasedPage {...data} product="cli" productDisplayName="Actyx CLI" downloads={DOWNLOADS} />
  )
}

export default Page
