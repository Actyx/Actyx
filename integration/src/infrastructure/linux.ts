import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import execa from 'execa'
import { CLI } from '../cli'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxOsDockerImage, actyxOsLinuxBinary, currentAxBinary } from './settings'
import { Ssh } from './ssh'
import { ActyxOSNode, printTarget, SshAble, Target } from './types'

// determines frequency of retrying ssh operations like connect()
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

const START_TIMEOUT = 60_000

export const mkNodeSshProcess = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void = console.log,
): Promise<ActyxOSNode> => {
  console.log('setting up ActyxOS process: %s on %o', nodeName, printTarget(target))

  if (target.os !== 'linux') {
    throw new Error(`mkNodeSshProces cannot install on OS ${target.os}`)
  }

  const ssh = new Ssh(sshParams.host, sshParams.username, sshParams.privateKey)
  await connectSsh(ssh, nodeName, sshParams)

  const binaryPath = await actyxOsLinuxBinary(target.arch)
  await uploadActyxOS(nodeName, ssh, binaryPath)

  const proc = await startActyxOS(nodeName, logger, ssh)

  return await forwardPortsAndBuildClients(ssh, nodeName, target, proc[0], {
    host: 'process',
  })
}

export const mkNodeSshDocker = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void,
  gitHash: string,
): Promise<ActyxOSNode> => {
  console.log('settings up ActyxOS on Docker: %s on %o', nodeName, printTarget(target))

  if (target.os !== 'linux') {
    throw new Error(`mkNodeSshDocker cannot install on OS ${target.os}`)
  }

  const ssh = new Ssh(sshParams.host, sshParams.username, sshParams.privateKey)
  await connectSsh(ssh, nodeName, sshParams)

  await ensureDocker(ssh, nodeName, sshParams.username)
  const userPass = await execa('vault', [
    'kv',
    'get',
    '--format=json',
    'secret/ops.actyx.dockerhub.deployUser',
  ])
  if (userPass.exitCode !== 0) {
    throw new Error('cannot get dockerhub credentials - you need to be authenticated to vault')
  }
  const { user, pass } = JSON.parse(userPass.stdout).data.data
  await execSsh(ssh)(`docker login -u ${user} -p ${pass}`)
  console.log('node %s Docker login successful', nodeName)

  const command =
    'docker run -i --rm -e AX_DEV_MODE=1 -e ENABLE_DEBUG_LOGS=1 -v /data --privileged ' +
    '-p 4001:4001 -p 127.0.0.1:4458:4458 -p 127.0.0.1:4454:4454 -p 127.0.0.1:4243:4243 ' +
    actyxOsDockerImage(target.arch, gitHash)
  const proc = await startActyxOS(nodeName, logger, ssh, command)

  return await forwardPortsAndBuildClients(ssh, nodeName, target, proc[0], {
    host: 'docker',
  })
}

async function ensureDocker(ssh: Ssh, node: string, user: string) {
  try {
    const result = await ssh.exec('docker --version')
    if (result.exitCode === 0) {
      console.log('node %s Docker already installed', node)
      return
    }
  } catch (error) {
    // ignore and start installing
  }

  console.log('node %s installing Docker', node)
  const exec = execSsh(ssh)

  await exec('sudo apt update')
  console.log('node %s packages updated', node)

  await exec('sudo apt install -y docker.io')
  console.log('node %s Docker package installed', node)

  await exec(`sudo chgrp ${user} /var/run/docker.sock`)
  console.log('node %s permissions fixed', node)
}

function execSsh(ssh: Ssh) {
  return async (cmd: string) => {
    const result = await ssh.exec(cmd)
    if (result.exitCode !== 0) {
      console.error(result)
      throw result
    }
    return result.stdout
  }
}

async function connectSsh(ssh: Ssh, nodeName: string, sshParams: SshAble) {
  let connected = false
  let attempts = 15
  while (!connected && attempts-- > 0) {
    try {
      await pollDelay(() => execSsh(ssh)('true'))
      connected = true
    } catch (error) {
      if (
        error.stderr.indexOf('Connection refused') >= 0 ||
        error.stderr.indexOf('Connection timed out') >= 0
      ) {
        // this is expected
      } else {
        console.log(
          'node %s ssh connection error to %s (remaining attempts %i): %o',
          nodeName,
          sshParams.host,
          attempts,
          error,
        )
      }
    }
  }
  if (!connected) {
    console.log('node %s ssh connection unsuccessful', nodeName)
    throw new Error('cannot connect')
  }
  console.log('SSH connection open to %s', nodeName)
}

async function uploadActyxOS(nodeName: string, ssh: Ssh, binaryPath: string) {
  console.log('node %s installing ActyxOS', nodeName)
  await ssh.scp(binaryPath, 'actyxos')
}

function startActyxOS(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh,
  command = 'RUST_BACKTRACE=1 ./actyxos',
): Promise<[execa.ExecaChildProcess<string>]> {
  // awaiting a Promise<Promise<T>> yields T (WTF?!?) so we need to put it into an array
  return new Promise((res, rej) => {
    setTimeout(
      () =>
        rej(new Error(`node ${nodeName}: ActyxOS did not start within ${START_TIMEOUT / 1000}sec`)),
      START_TIMEOUT,
    )
    const { log, flush } = mkProcessLogger(logger, nodeName, ['ActyxOS ready', 'ActyxOS started'])
    const proc = ssh.exec(command)
    proc.stdout?.on('data', (s: Buffer | string) => {
      if (log('stdout', s)) {
        res([proc])
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', () => {
      flush()
      logger(`node ${nodeName} ActyxOS channel closed`)
      rej('closed')
    })
    proc.on('error', (err: Error) => {
      logger(`node ${nodeName} ActyxOS channel error: ${err}`)
      rej(err)
    })
    proc.on('exit', (code: number, signal: string) => {
      logger(`node ${nodeName} ActyxOS exited with code=${code} signal=${signal}`)
      rej('exited')
    })
  })
}

export const forwardPortsAndBuildClients = async (
  ssh: Ssh,
  nodeName: string,
  target: Target,
  actyxOsProc: execa.ExecaChildProcess<string> | undefined,
  theRest: Omit<ActyxOSNode, 'ax' | 'actyxOS' | '_private' | 'name' | 'target'>,
): Promise<ActyxOSNode> => {
  const [[port4243, port4454, port4458], proc] = await ssh.forwardPorts(4243, 4454, 4458)

  console.log('node %s console reachable on port %i', nodeName, port4458)
  console.log('node %s event service reachable on port %i', nodeName, port4454)
  console.log('node %s pond service reachable on port %i', nodeName, port4243)

  const axBinaryPath = await currentAxBinary()
  const axHost = `localhost:${port4458}`
  console.error('created cli w/ ', axHost)
  const ax = new CLI(axHost, axBinaryPath)

  const apiConsole = `http://localhost:${port4458}/api/`
  const apiEvent = `http://localhost:${port4454}/api/`
  const opts = DefaultClientOpts()
  opts.Endpoints.ConsoleService.BaseUrl = apiConsole
  opts.Endpoints.EventService.BaseUrl = apiEvent
  const actyxOS = Client(opts)

  const apiPond = `ws://localhost:${port4243}/store_api`

  const shutdown = async () => {
    console.log('node %s shutting down', nodeName)
    actyxOsProc?.kill('SIGTERM')
    console.log('node %s ssh stopped', nodeName)
    await target._private.cleanup()
    console.log('node %s instance terminated', nodeName)
    proc.kill('SIGTERM')
  }

  const _private = { shutdown, axBinaryPath, axHost, apiConsole, apiEvent, apiPond }
  return { ax, actyxOS, _private, name: nodeName, target, ...theRest }
}
