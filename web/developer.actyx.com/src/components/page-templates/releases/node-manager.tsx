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
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-amd64.tar.gz`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-arm64.tar.gz`,
      },
      {
        arch: 'armhf',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-armhf.tar.gz`,
      },
      {
        arch: 'arm',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-linux-arm.tar.gz`,
      },
    ]
  },
  {

    platform: "Mac",
    ext: ".zip",
    files: [
      {
        arch: 'intel',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-macos-intel.zip`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-macos-arm.zip`,
      },
    ]
  },
  {
    platform: "Windows (Installer)", ext: ".zip",
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-node-manager-$V-installer-windows-x64.zip`,
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
  }
}> = ({ data }) => {
  return (
    <FileBasedPage {...data} product="node-manager" productDisplayName="Actyx Node Manager" downloads={DOWNLOADS} />
  )
}

export default Page
