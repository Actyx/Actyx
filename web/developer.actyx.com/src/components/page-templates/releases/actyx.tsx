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
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-linux-amd64.tar.gz`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-linux-arm64.tar.gz`,
      },
      {
        arch: 'armhf',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-linux-armhf.tar.gz`,
      },
      {
        arch: 'arm',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-linux-arm.tar.gz`,
      },
    ]
  },
  {

    platform: "Mac",
    ext: ".zip",
    files: [
      {
        arch: 'intel',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-macos-intel.zip`,
      },
      {
        arch: 'arm64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-macos-arm.zip`,
      },
    ]
  },
  {
    platform: "Windows (Installer)", ext: ".zip",
    files: [
      {
        arch: 'x64',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-installer-windows-x64.zip`,
      },
    ]
  },
  {
    platform: "Android (APK)",
    ext: ".zip",
    files: [
      {
        arch: 'all',
        target: `https://axartifacts.blob.core.windows.net/artifacts/$C/actyx-$V-android.zip`,
      },
    ]
  }
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
    <FileBasedPage {...data} product="actyx" productDisplayName="Actyx" downloads={DOWNLOADS} />
  )
}

export default Page
