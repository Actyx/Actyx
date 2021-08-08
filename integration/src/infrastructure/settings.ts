import execa from 'execa'
import fs from 'fs'
import { ensureDirSync } from 'fs-extra'
import https from 'https'
import * as t from 'io-ts'
import { tmpdir } from 'os'
import path from 'path'
import { MyGlobal } from '../../jest/setup'
import { Arch, currentArch, currentOS, OS, Settings } from '../../jest/types'
import { archToDockerPlatform, DockerPlatform } from './linux'
import { randIdentifier } from './util'

const DockerSingleManifest = t.type({
  digest: t.string,
  platform: DockerPlatform,
})
type DockerSingleManifest = t.TypeOf<typeof DockerSingleManifest>

const DockerManifest = t.type({
  manifests: t.array(DockerSingleManifest),
})
type DockerManifest = t.TypeOf<typeof DockerManifest>

export const settings = (): Settings => (<MyGlobal>global).axNodeSetup.settings

// .exe will be appended in case target is windows
export const enum Binary {
  ax = 'ax',
  actyxLinux = 'actyx-linux',
  actyxInstaller = 'actyx-x64',
  actyxAndroid = 'actyx.apk',
}

export const currentAxBinary = (): Promise<string> => getCurrent(Binary.ax)
export const currentActyxBinary = (): Promise<string> => getCurrent(Binary.actyxLinux)

export const dotnetEventsCliAssembly = (): Promise<string> =>
  ensureBinaryExists(currentOS(), `../dist/dotnet/cli`).then((x) => `${x}/CLI.dll`)

export const dotnetIntegrationTestsAssembly = (): Promise<string> =>
  ensureBinaryExists(currentOS(), `../dist/dotnet/sdk-integration`).then(
    (x) => `${x}/Sdk.IntegrationTests.dll`,
  )

const getCurrent = (bin: Binary) =>
  settings().gitHash == null
    ? // TODO: Derive Binary from currentOS()
      ensureBinaryExists(currentOS(), `../dist/bin/current/${bin}`)
    : getOrDownload(currentOS(), currentArch(), bin, settings().gitHash)

export const actyxLinuxBinary = async (arch: Arch): Promise<string> =>
  getOrDownload('linux', arch, Binary.actyxLinux, settings().gitHash)

export const actyxCliWindowsBinary = async (arch: Arch): Promise<string> =>
  getOrDownload('windows', arch, Binary.ax, settings().gitHash)

// Extract the image for the architecture we want to test from the multiarch manifest. This is due to
// the fact that we have to use `aarch64` hosts to test `armv7` images.
export const actyxDockerImage = async (arch: Arch, version: string): Promise<string> => {
  const repo = 'actyx/cosmos'
  const dockerTag = `${repo}:actyx-${version}`
  const inspect = await execa.command(`docker manifest inspect ${dockerTag}`)
  const json = JSON.parse(inspect.stdout)

  return (
    DockerManifest.decode(json)
      .map(({ manifests }: DockerManifest) => {
        const targetPlatform = archToDockerPlatform(arch)
        const sha = manifests.find(
          ({ platform }: DockerSingleManifest) =>
            platform.architecture === targetPlatform.architecture &&
            platform.variant === targetPlatform.variant,
        )

        if (!sha) {
          throw `Image for taget platform ${targetPlatform} not found in docker tag ${dockerTag}`
        }

        return `${repo}@${sha.digest}`
      })
      // Assume that this is not a multi-arch manifest, but a single-arch image
      .getOrElse(`${repo}:actyx-${version}`)
  )
}

export const windowsActyxInstaller = async (arch: Arch): Promise<string> =>
  getOrDownload('windows', arch, Binary.actyxInstaller, settings().gitHash)

export const actyxAndroidApk = async (): Promise<string> =>
  getOrDownload('android', 'x86_64', Binary.actyxAndroid, settings().gitHash)

const ensureBinaryExists = async (os: OS, p: string): Promise<string> => {
  p = os === 'windows' ? `${p}.exe` : p
  if (!fs.existsSync(p)) {
    if (os === 'windows') {
      throw new Error('unable to make on Windows')
    }
    const cmd = `make ${path.relative('..', p)}`
    const cwd = path.resolve('..')
    console.log(`${p} doesn't exist. Running ${cmd} in ${cwd}. This might take a while.`)
    await execa.command(cmd, { cwd })
    console.log(`Successfully built ${p}`)
  }
  return p
}

const mutex: { [_: string]: Promise<unknown> | undefined } = {}

const getOrDownload = async (
  os: OS,
  arch: Arch,
  binary: Binary,
  gitHash: string | null,
): Promise<string> => {
  let localPath: string
  // TODO: remove hack (not all windows binaries end in .exe!)
  const bin = binary === 'actyx-x64' ? 'actyx-x64.msi' : os === 'windows' ? `${binary}.exe` : binary
  // actyx.apk sits in the root
  const id = os == 'android' ? '' : `/${os}-${arch}`
  if (gitHash !== null) {
    localPath = `../dist/bin/${gitHash}${id}/${bin}`
  } else {
    localPath = `../dist/bin${id}/${bin}`
  }

  while (!fs.existsSync(localPath)) {
    if (mutex[localPath]) {
      // `localPath` is already being downloaded or created. Waiting ..
      await mutex[localPath]
    } else {
      const p = new Promise((res, rej) => {
        ;(gitHash != null
          ? download(gitHash, os, arch, binary, localPath)
          : ensureBinaryExists(os, localPath)
        )
          .then(() => res())
          .catch((err) => rej(err))
      })
      mutex[localPath] = p
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
  const bin = binary === 'actyx-x64' ? 'actyx-x64.msi' : os === 'windows' ? `${binary}.exe` : binary
  // actyx.apk sits in the root
  const p = os == 'android' ? '' : `/${os}-${arch}`
  const url = `https://axartifacts.blob.core.windows.net/artifacts/${hash}/${os}-binaries${p}/${bin}`

  console.log('Downloading binary "%s" from "%s"', bin, url)

  const tmpFile = path.join(tmpdir(), `integration-${randIdentifier()}`)
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
