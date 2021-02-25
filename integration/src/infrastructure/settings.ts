import { MyGlobal } from '../../jest/setup'
import { Arch, Settings, OS, currentOS, currentArch } from '../../jest/types'
import execa from 'execa'
import fs from 'fs'
import path from 'path'
import https from 'https'
import { ensureDirSync } from 'fs-extra'

export const settings = (): Settings => (<MyGlobal>global).axNodeSetup.settings

// .exe will be appended in case target is windows
export const enum Binary {
  ax = 'ax',
  actyxOsLinux = 'actyx-linux',
  actyxOsInstaller = 'Actyx-Installer',
}

export const currentAxBinary = (): Promise<string> => getCurrent(Binary.ax)
export const currentActyxOsBinary = (): Promise<string> => getCurrent(Binary.actyxOsLinux)

const getCurrent = (bin: Binary) =>
  settings().gitHash == null
    ? // TODO: Derive Binary from currentOS()
      ensureBinaryExists(`../dist/bin/current/${bin}`)
    : getOrDownload(currentOS(), currentArch(), bin, settings().gitHash)

export const actyxOsLinuxBinary = async (arch: Arch): Promise<string> =>
  getOrDownload('linux', arch, Binary.actyxOsLinux, settings().gitHash)

// multiarch manifest, so it should Do The Right Thing (TM)
export const actyxOsDockerImage = (arch: Arch, version: string): string =>
  `actyx/cosmos:actyx-${version}`

export const windowsActyxOsInstaller = async (arch: Arch): Promise<string> =>
  getOrDownload('windows', arch, Binary.actyxOsInstaller, settings().gitHash)

const ensureBinaryExists = async (p: string): Promise<string> => {
  if (!fs.existsSync(p)) {
    const cmd = `make ${path.relative('..', p)}`
    const cwd = path.resolve('..')
    console.log(`${p} doesn't exist. Running ${cmd} in ${cwd}. This might take a while.`)
    await execa.command(cmd, { cwd })
    console.log(`Successfully built ${p}`)
  }
  return p
}

const getOrDownload = async (
  os: OS,
  arch: Arch,
  binary: Binary,
  gitHash: string | null,
): Promise<string> => {
  const bin = os == 'windows' ? `${binary}.exe` : binary
  const id = `${gitHash != null ? `${gitHash}-` : ''}${os}-${arch}`
  const localPath = `../dist/bin/${id}/${bin}`

  if (!fs.existsSync(localPath)) {
    await (gitHash != null
      ? download(gitHash, os, arch, binary, localPath)
      : ensureBinaryExists(localPath))
  }
  return Promise.resolve(localPath)
}

const download = (
  hash: string,
  os: OS,
  arch: Arch,
  binary: Binary,
  targetFile: string,
): Promise<string> => {
  const bin = os == 'windows' ? `${binary}.exe` : binary
  const url = `https://axartifacts.blob.core.windows.net/artifacts/${hash}/${os}-binaries/${os}-${arch}/${bin}`

  console.log('Downloading binary "%s" from "%s"', bin, url)

  ensureDirSync(path.dirname(targetFile))
  const file = fs.createWriteStream(targetFile, { mode: 0o755 })
  return new Promise((resolve, reject) =>
    https.get(url, (response) => {
      if (response.statusCode !== 200) {
        reject(`status code != 200: ${response.statusCode} (${response.statusMessage})`)
      }
      response.pipe(file)
      file
        .on('finish', () => {
          file.close()
        })
        .on('error', (err) => {
          fs.unlink(targetFile, () => ({})) // ignore error, file might have not existed in the first place
          reject(err)
        })
        .on('close', resolve)
    }),
  )
}

export const getTestName = (): string => {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions, @typescript-eslint/no-explicit-any
  const state = (<any>expect).getState()
  let testName: string = state.testPath
  if (testName.startsWith(process.cwd())) {
    testName = `<cwd>` + testName.substr(process.cwd().length)
  }
  testName += ': ' + state.currentTestName
  return testName
}
