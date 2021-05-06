import { MyGlobal } from '../../jest/setup'
import { Arch, Settings, OS, currentOS, currentArch } from '../../jest/types'
import execa from 'execa'
import fs from 'fs'
import path from 'path'
import https from 'https'
import { ensureDirSync } from 'fs-extra'
import { Observable } from 'rxjs'
import { tmpdir } from 'os'

export const settings = (): Settings => (<MyGlobal>global).axNodeSetup.settings

// .exe will be appended in case target is windows
export const enum Binary {
  ax = 'ax',
  actyxLinux = 'actyx-linux',
  actyxInstaller = 'Actyx-Installer',
  actyxAndroid = 'actyx.apk',
}

export const currentAxBinary = (): Promise<string> => getCurrent(Binary.ax)
export const currentActyxBinary = (): Promise<string> => getCurrent(Binary.actyxLinux)

const getCurrent = (bin: Binary) =>
  settings().gitHash == null
    ? // TODO: Derive Binary from currentOS()
      ensureBinaryExists(`../dist/bin/current/${bin}`)
    : getOrDownload(currentOS(), currentArch(), bin, settings().gitHash)

export const actyxLinuxBinary = async (arch: Arch): Promise<string> =>
  getOrDownload('linux', arch, Binary.actyxLinux, settings().gitHash)

// multiarch manifest, so it should Do The Right Thing (TM)
export const actyxDockerImage = (arch: Arch, version: string): string =>
  `actyx/cosmos:actyx-${version}`

export const windowsActyxInstaller = async (arch: Arch): Promise<string> =>
  getOrDownload('windows', arch, Binary.actyxInstaller, settings().gitHash)

export const actyxAndroidApk = async (): Promise<string> =>
  getOrDownload('android', 'x86_64', Binary.actyxAndroid, settings().gitHash)

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

const mutex: { [_: string]: boolean | undefined } = {}

const getOrDownload = async (
  os: OS,
  arch: Arch,
  binary: Binary,
  gitHash: string | null,
): Promise<string> => {
  const bin = os == 'windows' ? `${binary}.exe` : binary
  const id = `${gitHash != null ? `${gitHash}-` : ''}${os}-${arch}`
  // actyx.apk sits in the root
  const p = os == 'android' ? '' : `/${id}`
  const localPath = `../dist/bin${p}/${bin}`

  while (!fs.existsSync(localPath)) {
    if (mutex[localPath]) {
      // `localPath` is already being downloaded or created. Waiting ..
      await Observable.timer(500).first().toPromise()
    } else {
      mutex[localPath] = true
      await (gitHash != null
        ? download(gitHash, os, arch, binary, localPath)
        : ensureBinaryExists(localPath))
      delete mutex[localPath]
    }
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
  // actyx.apk sits in the root
  const p = os == 'android' ? '' : `/${os}-${arch}`
  const url = `https://axartifacts.blob.core.windows.net/artifacts/${hash}/${os}-binaries${p}/${bin}`

  console.log('Downloading binary "%s" from "%s"', bin, url)

  const tmpFile = path.join(tmpdir(), `integration-${Math.random().toString(36).substring(7)}`)
  const file = fs.createWriteStream(tmpFile, { mode: 0o755 })
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
          fs.unlinkSync(tmpFile)
          reject(err)
        })
        .on('close', () => {
          ensureDirSync(path.dirname(targetFile))
          fs.copyFileSync(tmpFile, targetFile)
          fs.unlinkSync(tmpFile)
          resolve()
        })
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
