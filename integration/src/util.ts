import { ExecaChildProcess, ExecaError, ExecaReturnValue } from 'execa'
import { CLI } from './cli'
import { runOnEvery } from './infrastructure/hosts'
import { mkProcessLogger, netString } from './infrastructure/mkProcessLogger'
import { Ssh } from './infrastructure/ssh'
import { ActyxNode } from './infrastructure/types'
import { waitForNodeToBeConfigured } from './retry'
import { mySuite, testName } from './tests/event-service/utils.support.test'

export const getHttpApi = (x: ActyxNode): string => x._private.httpApiOrigin

export const run = <T>(f: (httpApi: string) => Promise<T>): Promise<T[]> =>
  runOnEvery((node) => f(getHttpApi(node)))

export const randomString = (): string =>
  Math.random()
    .toString(36)
    .replace(/[^a-z]+/g, '')
    .substr(0, 5)

export const releases = {
  '1.1.5':
    'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96',
}

export const withContext = <T>(context: string, f: () => T): T => {
  try {
    return f()
  } catch (err) {
    err.message += `\n\ncontext:\n${context}`
    throw err
  }
}

export const binaryUrlAndNameForVersion = (
  node: ActyxNode,
  version: keyof typeof releases,
): [string, string] => {
  const basename = version.startsWith('1.') ? 'actyxos' : 'actyx'
  const baseUrl = releases[version]
  const { os, arch } = node.target
  switch (os) {
    case 'linux': {
      return [
        `${baseUrl}/linux-binaries/linux-${arch}/${basename}-linux`,
        `${basename}-linux-${version}`,
      ]
    }
    case 'windows': {
      return [
        `${baseUrl}/windows-binaries/windows-${arch}/${basename}.exe`,
        `${basename}-${version}.exe`,
      ]
    }
    default:
      throw new Error(`cannot get binaries for os=${os}`)
  }
}

export const randomBinds = ['--bind-admin', '0', '--bind-api', '0', '--bind-swarm', '0']
const randomBindsWin = randomBinds.map((x) => `'${x}'`).join(',')

export const runActyxVersion = async (
  node: ActyxNode,
  version: keyof typeof releases,
  workdir: string,
): Promise<ActyxProcess> => {
  const [url, baseExe] = binaryUrlAndNameForVersion(node, version)
  const exe = `${workdir}/${baseExe}`
  const v1 = version.startsWith('1.')
  const wd = v1 ? '--working_dir' : '--working-dir'
  const ts = new Date().toISOString()
  process.stdout.write(`${ts} node ${node.name} starting Actyx ${version} in workdir ${workdir}\n`)
  switch (node.target.os) {
    case 'linux': {
      await node.target.execute('mkdir', ['-p', workdir]).process
      const download = await node.target.execute('curl', ['-o', exe, url]).process
      if (download.exitCode !== 0) {
        console.log(`error downloading ${url}:`, download.stderr)
        throw new Error(`error downloading ${url}`)
      }
      await node.target.execute('chmod', ['+x', exe]).process
      return {
        ...node.target.execute(`./${exe}`, [wd, workdir].concat(v1 ? [] : randomBinds)),
        workdir,
      }
    }
    case 'windows': {
      const x = (s: string) => node.target.execute(s, [])
      await x(String.raw`New-Item -ItemType Directory -Path ${workdir} -Force`).process
      await x(String.raw`(New-Object System.Net.WebClient).DownloadFile('${url}','${exe}')`).process
      const cmd = String.raw`Start-Process -Wait -NoNewWindow -FilePath ${exe} -ArgumentList '${wd}','${workdir}'${
        v1 ? '' : ',' + randomBindsWin
      }`
      return { ...x(cmd), workdir }
    }
    default:
      throw new Error(`cannot run specific Actyx version on os=${node.target.os}`)
  }
}

export type ActyxProcess = {
  process: ExecaChildProcess
  workdir: string
  ssh?: Ssh
}

export const runActyx = async (
  node: ActyxNode,
  workdir: string | undefined,
  params: string[],
): Promise<ActyxProcess> => {
  const ts = new Date().toISOString()
  process.stdout.write(`${ts} node ${node.name} starting current Actyx in workdir ${workdir}\n`)
  switch (node.target.os) {
    case 'linux': {
      const exec = node.target.executeInContainer || node.target.execute
      workdir =
        workdir === undefined ? (await exec('mktemp -d', []).process).stdout.trim() : workdir
      const args = ['--working-dir', workdir].concat(params)
      return { ...exec(`./${node._private.actyxBinaryPath}`, args), workdir }
    }
    case 'windows': {
      workdir =
        workdir === undefined
          ? (
              await node.target.execute(
                String.raw`$tempFolderPath = Join-Path $Env:Temp $(New-Guid)
                  New-Item -Type Directory -Path $tempFolderPath | Out-Null
                  $out = Join-Path $tempFolderPath id
                  $out`,
                [],
              ).process
            ).stdout
          : workdir
      const argList = ['--working-dir', workdir, '--background']
        .concat(params)
        .map((x) => `'${x}'`)
        .join(',')
      const cmd = String.raw`Start-Process -Wait -NoNewWindow -FilePath "${node._private.actyxBinaryPath}" -ArgumentList ${argList}`
      return { ...node.target.execute(cmd, []), workdir }
    }
    default:
      throw new Error(`cannot start Actyx on os=${node.target.os}`)
  }
}

const getLog = async (
  proc: Promise<ActyxProcess>,
  triggers: string[],
  timeout: number,
): Promise<ExecaReturnValue | ExecaError | [string[], ExecaChildProcess, string, Ssh?]> => {
  const { process: p, workdir, ssh } = await proc
  return new Promise<ExecaReturnValue | ExecaError | [string[], ExecaChildProcess, string, Ssh?]>(
    (res) => {
      const logs: string[] = []
      setTimeout(() => res([logs, p, workdir, ssh]), timeout)
      const { log } = mkProcessLogger((s) => logs.push(s), '', triggers)
      const ts = () => `${new Date().toISOString()} node ${mySuite()} ${testName()}`
      p.stdout?.on('data', (buf) => {
        process.stderr.write(`${ts()} ${netString(buf)}`)
        if (log('stdout', buf)) {
          res([logs, p, workdir, ssh])
        }
      })
      p.stderr?.on('data', (buf) => {
        process.stderr.write(`${ts()} ${netString(buf)}`)
        if (log('stderr', buf)) {
          res([logs, p, workdir, ssh])
        }
      })
      p.stdout?.on('end', () => {
        process.stderr.write(`${ts()} ended\n`)
      })
      p.then(res, res)
    },
  )
}

/**
 * Run this process until
 *  - it stops voluntarily, either successfully or not (both resolving the proving)
 *  - it emits one of the trigger strings on stdout or stderr
 *  - it times out
 *
 * @param proc the process to monitor (will be killed in the end)
 * @param triggers strings upon which execution is considered to be done
 * @param timeout maximum time the process is allowed to run
 */
export const runUntil = async (
  proc: Promise<ActyxProcess>,
  triggers: string[],
  timeout: number,
): Promise<ExecaReturnValue | ExecaError | string[]> => {
  const result = await getLog(proc, triggers, timeout)
  if (Array.isArray(result)) {
    const [logs, p] = result
    p.kill()
    return logs
  }
  return result
}

const adminExtract = /ADMIN_API_BOUND: Admin API bound to \/ip[46]\/([^/]+)\/tcp\/([0-9]+)/
const swarmExtract = /SWARM_SERVICES_BOUND: Swarm Services bound to \/ip4\/([^/]+)\/tcp\/(\d+)/
const apiExtract = /API_BOUND: API bound to (.*):(\d+)\.$/

export type BoundTo = {
  admin: [string, number][]
  api: [string, number][]
  swarm: [string, number][]
  log: string
  process: ExecaChildProcess
  workdir: string
  ssh?: Ssh
}

export const retryWhileLockedOrBound = async (
  nodeName: string,
  tries: number,
  p: () => Promise<BoundTo>,
): Promise<BoundTo> => {
  let collisions = 100
  for (;;) {
    try {
      return await p()
    } catch (err) {
      if (/data directory .* is locked by another Actyx process/.test(`${err}`) && tries > 1) {
        tries -= 1
        await new Promise((res) => setTimeout(res, 1000))
      } else if (/ERR_PORT_COLLISION/.test(`${err}`) && collisions > 1) {
        // this is called only with randomBinds, so collisions are purely local Linux kernel race conditions
        // between opening, closing, and reopening a port ⇒ just retry
        collisions -= 1
        process.stderr.write(`retrying process creation for ${nodeName}`)
      } else {
        throw err
      }
    }
  }
}

export const startup = async (proc: Promise<ActyxProcess>): Promise<BoundTo> => {
  const result = await getLog(proc, ['NODE_STARTED_BY_HOST'], 10_000)
  if (!Array.isArray(result)) {
    throw new Error(`Actyx process didn’t start:\n${result.stderr}`)
  }
  const [logs, process, workdir, ssh] = result
  if (!logs.find((line) => line.includes('NODE_STARTED_BY_HOST'))) {
    throw new Error(`Actyx process lingered without success:\n${logs.join('\n')}`)
  }

  const info: BoundTo = {
    admin: [],
    api: [],
    swarm: [],
    log: logs.join('\n'),
    process,
    workdir,
    ssh,
  }

  for (const line of logs) {
    const admin = adminExtract.exec(line)
    admin && info.admin.push([admin[1], Number(admin[2])])
    const swarm = swarmExtract.exec(line)
    swarm && info.swarm.push([swarm[1], Number(swarm[2])])
    const api = apiExtract.exec(line)
    api && info.api.push([api[1], Number(api[2])])
  }

  return info
}

export const newProcess = async (node: ActyxNode, workingDir?: string): Promise<ActyxNode> => {
  const { process, workdir, ssh, ...bound } = await retryWhileLockedOrBound(
    `${node.name} ${mySuite()} ${testName()}`,
    workingDir ? 15 : 1,
    () => startup(runActyx(node, workingDir, randomBinds)),
  )
  const api = bound.api.find(([addr]) => addr === '127.0.0.1')?.[1]
  const admin = bound.admin.find(([addr]) => addr === '127.0.0.1')?.[1]
  if (api === undefined || admin === undefined) {
    process.kill()
    throw new Error(
      `some ports not bound on localhost: api=${JSON.stringify(bound.api)} admin=${JSON.stringify(
        bound.admin,
      )}`,
    )
  }
  const [[apiPort, adminPort], sshProcess] = ssh
    ? await ssh.forwardPorts(api, admin)
    : [[api, admin], undefined]
  const nodeName = `${node.name}-additional`
  global.process.stderr.write(
    `${new Date().toISOString()} ${nodeName} started, api=${apiPort} admin=${adminPort}, workdir=${workdir}\n`,
  )
  const axHost = `localhost:${adminPort}`
  const ax =
    node._private.workingDir === workingDir
      ? await CLI.buildWithIdentityPath(axHost, node._private.axBinaryPath, node.ax.identityPath)
      : await CLI.build(axHost, node._private.axBinaryPath)
  const newNode = {
    name: nodeName,
    target: node.target,
    host: node.host,
    ax,
    _private: {
      shutdown: async () => {
        await ax.internal.shutdown()
        process.kill()
        await process.catch(() => ({}))
        sshProcess?.kill()
      },
      actyxBinaryPath: node._private.actyxBinaryPath,
      workingDir: workdir,
      axBinaryPath: node._private.axBinaryPath,
      axHost,
      httpApiOrigin: `http://localhost:${apiPort}`,
      apiPond: '',
      apiSwarmPort: 0,
      apiEventsPort: apiPort,
    },
  }
  await waitForNodeToBeConfigured(newNode)
  return newNode
}

export const power_cycle = async (node: ActyxNode): Promise<void> => {
  const workdir = node._private.workingDir
  await node._private.shutdown()
  const n2 = await newProcess(node, workdir)
  node.ax = n2.ax
  node._private.shutdown = n2._private.shutdown
  node._private.axHost = n2._private.axHost
  node._private.httpApiOrigin = n2._private.httpApiOrigin
  node._private.apiEventsPort = n2._private.apiEventsPort
}
