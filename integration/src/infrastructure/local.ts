import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import execa from 'execa'
import { ensureDir, remove } from 'fs-extra'
import path from 'path'
import { CLI } from '../cli'
import { portInUse } from './checkPort'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxOsDockerImage, currentActyxOsBinary, currentAxBinary, settings } from './settings'
import { ActyxOSNode, Target } from './types'

let alreadyRunning: string | undefined = undefined

export const mkNodeLocalProcess = async (
  nodeName: string,
  target: Target,
  logger: (s: string) => void,
): Promise<ActyxOSNode> => {
  const workingDir = path.resolve(settings().tempDir, 'actyxos-data')
  await remove(workingDir)
  await ensureDir(workingDir)
  const binary = await currentActyxOsBinary()
  if (alreadyRunning !== undefined) {
    console.log(
      'node %s cannot start: local ActyxOS process already running for node %s',
      nodeName,
      alreadyRunning,
    )
    throw new Error('duplicate usage of local process')
  }
  alreadyRunning = nodeName
  console.log('node %s starting locally: %s in %s', nodeName, binary, workingDir)

  for (const port of [4001, 4243, 4454, 4458, 8080]) {
    if (await portInUse(port)) {
      throw new Error(`port ${port} is already in use`)
    }
  }

  const proc = execa(binary, ['--working-dir', workingDir], { env: { RUST_BACKTRACE: '1' } })
  const shutdown = async () => {
    console.log('node %s killing process', nodeName)
    proc.kill('SIGTERM')
  }
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

  await new Promise((res, rej) => {
    proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
    proc.on('error', rej)
    proc.on('exit', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
  }).catch((err) => {
    shutdown()
    flush()
    return Promise.reject(err)
  })
  console.log('node %s ActyxOS started', nodeName)

  const opts = DefaultClientOpts()
  const axBinaryPath = await currentAxBinary()
  return {
    name: nodeName,
    target,
    host: 'process',
    ax: await CLI.build('localhost:4458', axBinaryPath),
    actyxOS: Client(opts),
    _private: {
      shutdown,
      axBinaryPath,
      axHost: 'localhost',
      apiConsole: opts.Endpoints.ConsoleService.BaseUrl,
      apiEvent: opts.Endpoints.EventService.BaseUrl,
      apiPond: 'ws://localhost:4243/store_api',
    },
  }
}

export const mkNodeLocalDocker = async (
  nodeName: string,
  target: Target,
  gitHash: string,
  logger: (s: string) => void,
): Promise<ActyxOSNode> => {
  const image = actyxOsDockerImage(target.arch, gitHash)
  console.log('node %s starting on local Docker: %s', nodeName, image)

  // exposing the ports and then using -P to use random (free) ports, avoiding trouble
  const command =
    'docker run -d --rm -e AX_DEV_MODE=1 -e ENABLE_DEBUG_LOGS=1 -v /data --privileged ' +
    '--expose 4001 --expose 4458 --expose 4454 --expose 4243 -P ' +
    image

  const dockerRun = await execa.command(command)
  const container = dockerRun.stdout

  const shutdown = async () => {
    console.log('node %s shutting down container %s', nodeName, container)
    await execa('docker', ['stop', container])
  }

  try {
    const proc = execa('docker', ['logs', '--follow', container])
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

    await new Promise((res, rej) => {
      proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
      proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
      proc.on('close', (code: number, signal: string) =>
        rej(`channel closed, code: ${code}, signal: '${signal}'`),
      )
      proc.on('error', rej)
      proc.on('exit', (code: number, signal: string) =>
        rej(`channel closed, code: ${code}, signal: '${signal}'`),
      )
    }).catch((err) => {
      flush()
      return Promise.reject(err)
    })
    console.log('node %s ActyxOS started in container %s', nodeName, container)

    const dockerInspect = await execa('docker', ['inspect', container])
    const ports: { [p: string]: { HostIp: string; HostPort: string }[] } = JSON.parse(
      dockerInspect.stdout,
    )[0].NetworkSettings.Ports

    const port = (original: number): string => ports[`${original}/tcp`][0].HostPort
    const axHost = `localhost:${port(4458)}`
    const apiConsole = `http://localhost:${port(4454)}/api/`
    const apiEvent = `http://localhost:${port(4454)}/api/`
    const opts = DefaultClientOpts()
    opts.Endpoints.ConsoleService.BaseUrl = apiConsole
    opts.Endpoints.EventService.BaseUrl = apiEvent

    const axBinaryPath = await currentAxBinary()
    return {
      name: nodeName,
      target,
      host: 'docker',
      ax: await CLI.build(axHost, axBinaryPath),
      actyxOS: Client(opts),
      _private: {
        shutdown,
        axBinaryPath,
        axHost,
        apiConsole,
        apiEvent,
        apiPond: `ws://localhost:${port(4243)}/store_api`,
      },
    }
  } catch (err) {
    shutdown()
    throw err
  }
}
