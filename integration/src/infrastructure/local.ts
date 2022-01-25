import execa from 'execa'
import { ensureDir, remove } from 'fs-extra'
import path from 'path'
import { CLI } from '../cli'
import { getFreePort } from './checkPort'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxDockerImage, currentActyxBinary, currentAxBinary, settings } from './settings'
import { ActyxNode, Target } from './types'
import { mkLog } from './util'

export const mkNodeLocalProcess = (
  nodeName: string,
  target: Target,
  reuseWorkingDirIfExists?: boolean,
) => async (logger: (s: string) => void): Promise<ActyxNode> => {
  const clog = mkLog(nodeName)
  const workingDir = path.resolve(settings().tempDir, `${nodeName}-actyx-data`)
  if (reuseWorkingDirIfExists !== true) {
    await remove(workingDir)
  }
  await ensureDir(workingDir)
  const binary = await currentActyxBinary()

  clog(`starting locally: ${binary} in ${workingDir}`)

  const [port4001, port4454, port4458] = await Promise.all([0, 0, 0].map(() => getFreePort()))

  const proc = execa(
    binary,
    [
      '--working-dir',
      workingDir,
      '--bind-admin',
      `0.0.0.0:${port4458}`,
      '--bind-api',
      `0.0.0.0:${port4454}`,
      '--bind-swarm',
      `0.0.0.0:${port4001}`,
    ],
    { env: { RUST_BACKTRACE: '1' } },
  )
  const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

  await new Promise<void>((res, rej) => {
    proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s) && res())
    proc.on('close', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
    proc.on('error', rej)
    proc.on('exit', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
  }).catch((err) => {
    clog('killing process due to error')
    proc.kill('SIGTERM')
    flush()
    return Promise.reject(err)
  })

  proc.removeAllListeners('exit')
  const shutdown = (): Promise<void> => {
    if (proc.killed) {
      return Promise.resolve()
    }
    clog('shutdown process')
    proc.kill('SIGTERM')
    return new Promise<void>((resolve) =>
      proc.on('exit', (code: number, signal: string) => {
        clog(`channel closed, code: ${code}, signal: '${signal}'`)
        resolve()
      }),
    )
  }

  clog('Actyx started')
  clog(`admin reachable on port ${port4458}`)
  clog(`http api reachable on port ${port4454}`)

  const axBinaryPath = await currentAxBinary()
  const ax = await CLI.build(`localhost:${port4458}`, axBinaryPath)

  return {
    name: nodeName,
    target,
    host: 'process',
    ax,
    _private: {
      shutdown,
      actyxBinaryPath: binary,
      workingDir,
      axBinaryPath,
      hostname: 'localhost',
      adminPort: port4458,
      swarmPort: port4001,
      apiPort: port4454,
    },
  }
}

export const mkNodeLocalDocker = async (
  nodeName: string,
  target: Target,
  gitHash: string,
  logger: (s: string) => void,
): Promise<ActyxNode> => {
  const clog = mkLog(nodeName)
  const image = await actyxDockerImage(target.arch, gitHash)
  clog(`starting on local Docker: ${image}`)

  // exposing the ports and then using -P to use random (free) ports, avoiding trouble
  const command =
    'docker run -d --rm -v /data --expose 4001 --expose 4458 --expose 4454 -P ' + image

  const dockerRun = await execa.command(command)
  const container = dockerRun.stdout

  const shutdown = async () => {
    clog(`shutting down container ${container}`)
    await execa('docker', ['stop', container])
  }

  try {
    const proc = execa('docker', ['logs', '--follow', container])
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

    await new Promise<void>((res, rej) => {
      proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
      proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s) && res())
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
    clog(`Actyx started in container ${container}`)

    const dockerInspect = await execa('docker', ['inspect', container])
    const ports: { [p: string]: { HostIp: string; HostPort: string }[] } = JSON.parse(
      dockerInspect.stdout,
    )[0].NetworkSettings.Ports

    const port = (original: number): string => ports[`${original}/tcp`][0].HostPort

    const axBinaryPath = await currentAxBinary()
    const ax = await CLI.build(`localhost:${port(4458)}`, axBinaryPath)

    const executeInContainer = (script: string) =>
      target.execute(`docker exec ${container} ${script}`, [])
    return {
      name: nodeName,
      target: { ...target, executeInContainer },
      host: 'docker',
      ax,
      _private: {
        shutdown,
        axBinaryPath,
        actyxBinaryPath: 'actyx',
        workingDir: '/data/actyx-data',
        hostname: 'localhost',
        adminPort: Number(port(4458)),
        swarmPort: 4001,
        apiPort: Number(port(4454)),
      },
    }
  } catch (err) {
    shutdown()
    throw err
  }
}
