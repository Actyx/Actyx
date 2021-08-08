import { Download } from './types'

export const ACTYX_DOWNLOADS_V1_1_5: Download[] = [
  {
    platform: 'Linux',
    ext: 'executable',
    files: [
      {
        arch: 'amd64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-x86_64/actyxos-linux',
      },
      {
        arch: 'aarch64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-aarch64/actyxos-linux',
      },
    ],
  },
  {
    platform: 'Mac',
    ext: 'executable',
    files: [
      {
        arch: 'intel',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0012_manual_actyxos-mac-1.1.5/actyxos-mac',
      },
    ],
  },
  {
    platform: 'Windows (Installer)',
    ext: '.zip',
    files: [
      {
        arch: 'x64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/windows-binaries/windows-x86_64/ActyxOS-Installer.exe',
      },
    ],
  },
  {
    platform: 'Android (APK)',
    ext: '.zip',
    files: [
      {
        arch: 'all',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/android-binaries/actyxos.apk',
      },
    ],
  },
]

export const CLI_DOWNLOADS_V1_1_5: Download[] = [
  {
    platform: 'Linux',
    ext: 'executable',
    files: [
      {
        arch: 'amd64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-x86_64/ax',
      },
      {
        arch: 'aarch64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-aarch64/ax',
      },
      {
        arch: 'armhf',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-armv7/ax',
      },
      {
        arch: 'arm',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/linux-binaries/linux-arm/ax',
      },
    ],
  },
  {
    platform: 'Mac',
    ext: 'executable',
    files: [
      {
        arch: 'intel',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0013_manual_1.1.5_actyx-cli/ax',
      },
    ],
  },
  {
    platform: 'Windows',
    ext: '.exe',
    files: [
      {
        arch: 'x64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96/windows-binaries/windows-x86_64/ax.exe',
      },
    ],
  },
]

export const NODE_MANAGER_DOWNLOADS_V1_1_5: Download[] = [
  {
    platform: 'Linux',
    ext: '.rpm',
    files: [
      {
        arch: 'amd64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0011_manual_1.1.5_node_manager/actyxos-node-manager-1.1.5-1.x86_64.rpm',
      },
    ],
  },
  {
    platform: 'Linux',
    ext: '.deb',
    files: [
      {
        arch: 'amd64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0011_manual_1.1.5_node_manager/actyxos-node-manager_1.1.5_amd64.deb',
      },
    ],
  },
  {
    platform: 'Linux',
    ext: '.zip',
    files: [
      {
        arch: 'amd64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0011_manual_1.1.5_node_manager/ActyxOS-Node-Manager-linux-x64-1.1.5.zip',
      },
    ],
  },
  {
    platform: 'Mac',
    ext: '.zip',
    files: [
      {
        arch: 'intel',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0011_manual_1.1.5_node_manager/ActyxOS-Node-Manager-darwin-x64-1.1.5.zip',
      },
    ],
  },
  {
    platform: 'Windows (Installer)',
    ext: '.exe',
    files: [
      {
        arch: 'x64',
        target:
          'https://axartifacts.blob.core.windows.net/artifacts/0011_manual_1.1.5_node_manager/ActyxOS-Node-Manager-1.1.5%20Setup.exe',
      },
    ],
  },
]
