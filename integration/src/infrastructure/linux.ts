import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import execa from 'execa'
import { CLI } from '../cli'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxOsDockerImage, actyxOsLinuxBinary, currentAxBinary } from './settings'
import * as Ssh from './ssh'
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

  const ssh = new Ssh.Client(sshParams)
  await connectSsh(ssh, nodeName, sshParams)

  const binaryPath = actyxOsLinuxBinary(target.arch)
  await uploadActyxOS(nodeName, ssh, binaryPath)

  await startActyxOS(nodeName, logger, ssh)

  const clients = await forwardPortsAndBuildClients(ssh, logger, nodeName, target)
  return {
    name: nodeName,
    target,
    host: 'process',
    runtimes: [],
    ...clients,
  }
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

  const ssh = new Ssh.Client(sshParams)
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
    '-p 4001:4001 -p 127.0.0.1:4457:4457 -p 127.0.0.1:4454:4454 -p 127.0.0.1:4243:4243 ' +
    actyxOsDockerImage(target.arch, gitHash)
  await startActyxOS(nodeName, logger, ssh, command)

  const clients = await forwardPortsAndBuildClients(ssh, logger, nodeName, target)
  return {
    name: nodeName,
    target,
    host: 'docker',
    runtimes: ['docker'],
    ...clients,
  }
}

async function ensureDocker(ssh: Ssh.Client, node: string, user: string) {
  const result = await ssh.exec('docker --version')
  if (result.code === 0) {
    console.log('node %s Docker already installed', node)
    return
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

function execSsh(ssh: Ssh.Client) {
  return async (cmd: string) => {
    // the `exit $?` is appended to ensure that the process exits correctly
    const result = await ssh.exec(cmd + '; exit $?')
    if (result.code !== 0) {
      console.error(result)
      throw new Error(`error when running ${cmd}`)
    }
    return result.stdout
  }
}

async function connectSsh(ssh: Ssh.Client, nodeName: string, sshParams: SshAble) {
  let connected = false
  let attempts = 15
  while (!connected && attempts-- > 0) {
    try {
      await pollDelay(() => ssh.connect())
      connected = true
    } catch (error) {
      if (error.code !== 'ECONNREFUSED' && error.level !== 'client-timeout') {
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
    console.log('node %s ssh connection refused', nodeName)
    throw new Error('connection refused')
  }
  console.log('SSH connection open to %s', nodeName)
}

async function uploadActyxOS(nodeName: string, ssh: Ssh.Client, binaryPath: string) {
  console.log('node %s installing ActyxOS', nodeName)
  await ssh.sftp(async (sftp) => {
    // ignore errors for unlink
    await Ssh.mkProm0((cb) => {
      setTimeout(() => cb(new Error('timed out while unlinking ActyxOS')), 10_000)
      sftp.unlink('actyxos', () => cb(undefined))
    })
    await Ssh.mkProm0((cb) => {
      setTimeout(() => cb(new Error('timed out while uploading ActyxOS')), 30_000)
      sftp.fastPut(
        binaryPath,
        'actyxos',
        {
          mode: 0o755,
          step: (curr, chunk, total) => {
            const granularity = 0.1
            const now = curr / total
            const prev = (curr - chunk) / total
            const steps = Math.floor(now / granularity)
            if (prev < steps * granularity) {
              console.log('node %s ActyxOS installed %i%%', nodeName, Math.floor(100 * now))
            }
          },
          concurrency: 1,
          chunkSize: 1_048_576,
        },
        cb,
      )
    })
  })
}

async function startActyxOS(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh.Client,
  command = './actyxos',
) {
  await new Promise<void>((res, rej) => {
    setTimeout(
      () =>
        rej(new Error(`node ${nodeName}: ActyxOS did not start within ${START_TIMEOUT / 1000}sec`)),
      START_TIMEOUT,
    )
    const { log, flush } = mkProcessLogger(logger, nodeName)
    ssh.conn.exec(command, { env: { RUST_BACKTRACE: '1' } }, (err, channel) => {
      if (err) rej(err)
      channel.on('data', (s: Buffer | string) => {
        if (log('stdout', s)) {
          res()
        }
      })
      channel.stderr.on('data', (s: Buffer | string) => log('stderr', s))
      channel.on('close', () => {
        flush()
        logger(`node ${nodeName} ActyxOS channel closed`)
        rej('closed')
      })
      channel.on('error', (err: Error) => {
        logger(`node ${nodeName} ActyxOS channel error: ${err}`)
        rej(err)
      })
    })
  })
}

async function forwardPortsAndBuildClients(
  ssh: Ssh.Client,
  logger: (s: string) => void,
  nodeName: string,
  target: Target,
) {
  const [port4457, server4457] = await ssh.forwardPort(4457, (line) =>
    logger(`node ${nodeName} ${line}`),
  )
  console.log('node %s console reachable on port %i', nodeName, port4457)

  const [port4454, server4454] = await ssh.forwardPort(4454, (line) =>
    logger(`node ${nodeName} ${line}`),
  )
  console.log('node %s event service reachable on port %i', nodeName, port4454)

  const [port4243, server4243] = await ssh.forwardPort(4243, (line) =>
    logger(`node ${nodeName} ${line}`),
  )
  console.log('node %s pond service reachable on port %i', nodeName, port4243)

  const axBinaryPath = currentAxBinary
  const axHost = `localhost:${port4457}`
  const ax = new CLI(axHost, axBinaryPath)

  const apiConsole = `http://localhost:${port4457}/api/`
  const apiEvent = `http://localhost:${port4454}/api/`
  const opts = DefaultClientOpts()
  opts.Endpoints.ConsoleService.BaseUrl = apiConsole
  opts.Endpoints.EventService.BaseUrl = apiEvent
  const actyxOS = Client(opts)

  const apiPond = `ws://localhost:${port4243}/store_api`

  const shutdown = async () => {
    console.log('node %s shutting down', nodeName)
    server4454.emit('end')
    server4457.emit('end')
    server4243.emit('end')
    await ssh.end()
    console.log('node %s ssh stopped', nodeName)
    await target._private.shutdown()
    console.log('node %s instance terminated', nodeName)
  }

  const _private = { shutdown, axBinaryPath, axHost, apiConsole, apiEvent, apiPond }
  return { ax, actyxOS, _private }
}
