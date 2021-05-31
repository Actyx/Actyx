import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import execa from 'execa'
import { CLI } from '../cli'
import { awaitCloudInitSetup } from './aws'
import { connectSsh, ensureDocker, execSsh } from './linux'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxAndroidApk, currentAxBinary } from './settings'
import { Ssh } from './ssh'
import { ActyxNode, printTarget, SshAble, Target } from './types'
import { mkLog } from './util'

export const mkNodeSshAndroid = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void,
): Promise<ActyxNode> => {
  const log = mkLog(nodeName)
  log('setting up Actyx on Android on Docker on Linux: %s on %o', nodeName, printTarget(target))

  if (target.os !== 'linux') {
    throw new Error(`mkNodeSshAndroid cannot install on OS ${target.os}`)
  }
  if (target.arch !== 'x86_64') {
    throw new Error(`mkNodeSshAndroid cannot install on ${target.arch}`)
  }

  const ssh = Ssh.new(sshParams.host, sshParams.username, sshParams.privateKey)
  await connectSsh(ssh, nodeName, sshParams, 150)

  if (target.kind.type === 'aws') {
    await awaitCloudInitSetup(ssh)
  }
  await ensureDocker(ssh, nodeName, target.arch)

  const exec = execSsh(ssh)
  try {
    await exec('adb version')
    log('adb already installed')
  } catch (e) {
    await exec('sudo apt install adb -y')
  }
  try {
    // make sure adb private key is written to disk
    await exec('adb get-state')
  } catch (e) {
    // ignore
  }

  const cmd = String.raw`docker run -d --rm \
  -e ADBKEY="$(cat ~/.android/adbkey)" \
  --device /dev/kvm \
  --publish 5555 \
  actyx/util:androidemulator-google-x86-no-metrics-latest`

  const container = await exec(cmd)
  log('Pulling and starting emulator')
  const dockerLogsProc = (
    await runProcess(nodeName, logger, ssh, `docker logs --follow ${container}`, [
      'emulator: INFO: boot completed',
    ])
  )[0]
  log('Emulator started')

  const dockerInspect = await exec(`docker inspect ${container}`)
  const adbPort = JSON.parse(dockerInspect)[0].NetworkSettings.Ports['5555/tcp'][0].HostPort

  const apk = '/tmp/actyx.apk'
  await ssh.scp(await actyxAndroidApk(), apk)
  await exec(`adb connect localhost:${adbPort}`)
  const execAdb = (command: string) => exec(`adb -s localhost:${adbPort} ${command}`)
  await execAdb('wait-for-device')

  const [remotePort4001, remotePort4454, remotePort4458] = await Promise.all(
    [4001, 4454, 4458].map((x) => execAdb(`forward tcp:0 tcp:${x}`).then(Number)),
  )

  const [[port4454, port4458], sshProc] = await ssh.forwardPorts(remotePort4454, remotePort4458)
  log('admin reachable on port %i', port4458)
  log('http api reachable on port %i', port4454)

  log('Starting Actyx on Android')
  await execAdb(`install ${apk}`)
  await new Promise((r) => setTimeout(r, 2000))

  let retry = 0
  while (retry < 10) {
    const waitForIt = runProcess(
      nodeName,
      logger,
      ssh,
      `docker logs -n 1 --follow ${container}`,
      ['NODE_STARTED_BY_HOST'],
      10000,
    ).then((x) => {
      x.forEach((p) => p.kill('SIGTERM'))
    })
    await execAdb('shell am start -n com.actyx.android/com.actyx.android.MainActivity')
    try {
      await waitForIt
      break
    } catch (e) {
      // TODO: On first start, it seems the background service is not started
      // properly.  Not sure yet whether this is a fluke with the emulator or with
      // Actyx
      await new Promise((r) => setTimeout(r, 1000))
      await execAdb('shell am force-stop com.actyx.android')
      await new Promise((r) => setTimeout(r, 1000))
      retry++
      if (retry++ > 5) {
        throw e
      }
    }
  }
  log('Actyx on Android started')

  const axHost = `localhost:${port4458}`
  const httpApiOrigin = `http://localhost:${port4454}`
  const opts = DefaultClientOpts()
  opts.Endpoints.EventService.BaseUrl = httpApiOrigin

  const axBinaryPath = await currentAxBinary()
  const shutdown = async () => {
    log(`shutting down container ${container}`)
    dockerLogsProc.kill('SIGTERM')
    await exec(`docker stop ${container}`)
    sshProc.kill('SIGTERM')
    await target._private.cleanup()
  }

  const executeInContainerPrefix = `adb -s localhost:${adbPort} `
  const executeInContainer = (script: string) =>
    target.execute(`${executeInContainerPrefix}${script}`, [], {})
  return {
    name: nodeName,
    target: {
      ...target,
      _private: { ...target._private, executeInContainerPrefix },
      executeInContainer,
    },
    ax: await CLI.build(axHost, axBinaryPath),
    httpApiClient: Client(opts),
    host: 'android',
    _private: {
      shutdown,
      actyxBinaryPath: '',
      axBinaryPath,
      axHost,
      httpApiOrigin,
      apiPond: `ws://localhost:${port4454}/api/v2/events`,
      apiSwarmPort: remotePort4001,
      apiEventsPort: port4454,
    },
  }
}
export function runProcess(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh,
  command: string,
  signal: string[],
  startTimeoutMs = 180000,
): Promise<[execa.ExecaChildProcess<string>]> {
  // awaiting a Promise<Promise<T>> yields T (WTF?!?) so we need to put it into an array
  return new Promise((res, rej) => {
    setTimeout(
      () =>
        rej(new Error(`node ${nodeName}: cmd did not yield within ${startTimeoutMs / 1000}sec`)),
      startTimeoutMs,
    )
    const { log, flush } = mkProcessLogger(logger, nodeName, signal)
    const proc = ssh.exec(command)
    proc.stdout?.on('data', (s: Buffer | string) => {
      if (log('stdout', s)) {
        res([proc])
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', () => {
      flush()
      logger(`node ${nodeName} cmd channel closed`)
      rej('closed')
    })
    proc.on('error', (err: Error) => {
      logger(`node ${nodeName} cmd channel error: ${err}`)
      rej(err)
    })
    proc.on('exit', (code: number, sig: string) => {
      logger(`node ${nodeName} cmd exited with code=${code} signal=${sig}`)
      rej('exited')
    })
  })
}
