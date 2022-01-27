/* eslint-disable @typescript-eslint/explicit-module-boundary-types */
import { execa, ExecaChildProcess } from 'execa'
import * as t from 'io-ts'
import { Arch } from '../jest/types'
import { CLI } from '../cli'
import { awaitCloudInitSetup } from './aws'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxDockerImage, actyxLinuxBinary, currentAxBinary } from './settings'
import { Ssh } from './ssh'
import { ActyxNode, printTarget, SshAble, Target } from './types'
import { mkLog } from './util'

// determines frequency of retrying ssh operations like connect()
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

const START_TIMEOUT = 60_000

export const DockerPlatform = t.type({
  architecture: t.string,
  variant: t.union([t.string, t.undefined]),
})
export type DockerPlatform = t.TypeOf<typeof DockerPlatform>

export const mkNodeSshProcess = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void = console.log,
): Promise<ActyxNode> => {
  console.log('setting up Actyx process: %s on %o', nodeName, printTarget(target))

  if (target.os !== 'linux') {
    throw new Error(`mkNodeSshProcess cannot install on OS ${target.os}`)
  }

  const ssh = Ssh.new(sshParams.host, sshParams.username, sshParams.privateKey)
  await connectSsh(ssh, nodeName, sshParams)

  // Install test dependencies
  if (target.kind.type === 'aws') {
    await awaitCloudInitSetup(ssh)
    console.log(`node ${nodeName} cloudInitSetup done`)
  }
  await execSsh(ssh)('sudo apt update && sudo apt install ncat -y ')
  console.log(`node ${nodeName} ncat installed`)

  const binaryPath = await actyxLinuxBinary(target.arch)
  await uploadActyx(nodeName, ssh, binaryPath)

  const proc = await startActyx(nodeName, logger, ssh)

  return await forwardPortsAndBuildClients(ssh, nodeName, target, proc, 'actyx-data', {
    host: 'process',
  })
}

export const mkNodeSshDocker = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void,
  gitHash: string,
): Promise<ActyxNode> => {
  console.log('setting up Actyx on Docker: %s on %o', nodeName, printTarget(target))

  if (target.os !== 'linux') {
    throw new Error(`mkNodeSshDocker cannot install on OS ${target.os}`)
  }

  const ssh = Ssh.new(sshParams.host, sshParams.username, sshParams.privateKey)
  await connectSsh(ssh, nodeName, sshParams)

  if (target.kind.type === 'aws') {
    await awaitCloudInitSetup(ssh)
  }
  await ensureDocker(ssh, nodeName, target.arch)
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
    'docker run -i --rm -v /data ' +
    '-p 4001:4001 -p 127.0.0.1:4458:4458 -p 127.0.0.1:4454:4454 ' +
    (await actyxDockerImage(target.arch, gitHash))
  const proc = await startActyx(nodeName, logger, ssh, command)

  // TODO: Support multiple containers on the same host, and fill
  // `target.executeInContainer`
  return await forwardPortsAndBuildClients(ssh, nodeName, target, proc, '/data/actyx-data', {
    host: 'docker',
  })
}

const archToDockerHostType = (arch: Arch): string => {
  switch (arch) {
    case 'aarch64':
      return 'arm64'
    case 'arm':
      throw new Error('Arm is not supported')
    case 'armv7':
      return 'arm64'
    case 'x86_64':
      return 'amd64'
  }
}

export const archToDockerPlatform = (arch: Arch): DockerPlatform => {
  switch (arch) {
    case 'aarch64':
      return { architecture: 'arm64', variant: undefined }
    case 'arm':
      return { architecture: 'arm', variant: 'v6' }
    case 'armv7':
      return { architecture: 'arm', variant: 'v7' }
    case 'x86_64':
      return { architecture: 'amd64', variant: undefined }
  }
}

/**
 * Install Docker. This procedure is dependant on the `ami` specified in hosts.yaml
 */
export async function ensureDocker(ssh: Ssh, node: string, arch: Arch) {
  const log = mkLog(node)
  try {
    const result = await ssh.exec('docker --version')
    if (result.exitCode === 0) {
      log('Docker already installed')
      return
    }
  } catch (error) {
    // ignore and start installing
  }

  log('installing Docker')
  const exec = execSsh(ssh)

  // Procedure for installing https://docs.docker.com/engine/install/ubuntu/
  try {
    await exec('sudo apt-get remove --yes docker docker-engine docker.io containerd runc')
  } catch (x) {
    // It’s OK if apt-get reports that none of these packages are installed.
  }
  await exec('sudo apt-get update')
  await exec(
    'sudo apt-get --yes install apt-transport-https ca-certificates curl gnupg lsb-release',
  )
  await exec(
    'curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg',
  )
  await exec(
    `echo "deb [arch=${archToDockerHostType(
      arch,
    )} signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null`,
  )
  await exec('sudo apt-get update')
  // Test against well known versions
  const dckr_v = '5:20.10.6~3-0~ubuntu-hirsute'
  const cntnrd_v = '1.4.4-1'
  await exec(
    `sudo apt-get --yes install docker-ce=${dckr_v} docker-ce-cli=${dckr_v} containerd.io=${cntnrd_v}`,
  )

  // Fix `Got permission denied while trying to connect to the Docker daemon socket`
  await exec(`sudo chgrp $USER /var/run/docker.sock`)
  log('Docker installed')
}

export function execSsh(ssh: Ssh) {
  return async (cmd: string) => {
    const result = await ssh.exec(cmd)
    if (result.exitCode !== 0) {
      throw result
    }
    return result.stdout
  }
}

export async function connectSsh(ssh: Ssh, nodeName: string, sshParams: SshAble, maxAttempts = 25) {
  let connected = false
  let attempts = maxAttempts
  while (!connected && attempts-- > 0) {
    try {
      await pollDelay(() => execSsh(ssh)('whoami'))
      connected = true
    } catch (error) {
      if (
        (error as any).stderr &&
        ((error as any).stderr.indexOf('Connection refused') >= 0 ||
          (error as any).stderr.indexOf('Connection timed out') >= 0 ||
          (error as any).stderr.indexOf('Operation timed out') >= 0)
      ) {
        console.log(
          'node %s ssh connection (%s) not yet ready (remaining attempts %i)',
          nodeName,
          sshParams.host,
          attempts,
        )
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

async function uploadActyx(nodeName: string, ssh: Ssh, binaryPath: string) {
  console.log('node %s installing Actyx %s', nodeName, binaryPath)
  await ssh.scp(binaryPath, 'actyx')
  console.log(`node ${nodeName} Actyx installed`)
}

export function startActyx(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh,
  command = 'RUST_BACKTRACE=1 ./actyx',
): Promise<[ExecaChildProcess<string>]> {
  // awaiting a Promise<Promise<T>> yields T (WTF?!?) so we need to put it into an array
  return new Promise((res, rej) => {
    setTimeout(
      () =>
        rej(new Error(`node ${nodeName}: Actyx did not start within ${START_TIMEOUT / 1000}sec`)),
      START_TIMEOUT,
    )
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])
    const proc = ssh.exec(command)
    proc.stdout?.on('data', (s: Buffer | string) => {
      if (log('stdout', s)) {
        res([proc])
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => {
      if (log('stderr', s)) {
        res([proc])
      }
    })
    proc.on('close', () => {
      flush()
      logger(`node ${nodeName} Actyx channel closed`)
      rej('closed')
    })
    proc.on('error', (err: Error) => {
      logger(`node ${nodeName} Actyx channel error: ${err}`)
      rej(err)
    })
    proc.on('exit', (code: number, signal: string) => {
      logger(`node ${nodeName} Actyx exited with code=${code} signal=${signal}`)
      rej('exited')
    })
  })
}

export const forwardPortsAndBuildClients = async (
  ssh: Ssh,
  nodeName: string,
  target: Target,
  actyxProc: ExecaChildProcess<string>[],
  workingDir: string,
  theRest: Omit<ActyxNode, 'ax' | '_private' | 'name' | 'target'>,
): Promise<ActyxNode> => {
  const [[port4454, port4458], proc] = await ssh.forwardPorts(4454, 4458)

  console.log('node %s admin reachable on port %i', nodeName, port4458)
  console.log('node %s http api reachable on port %i', nodeName, port4454)

  const axBinaryPath = await currentAxBinary()
  const ax = await CLI.build(`localhost:${port4458}`, axBinaryPath)

  const shutdown = async () => {
    console.log('node %s shutting down', nodeName)
    actyxProc.forEach((x) => x.kill('SIGTERM'))
    console.log('node %s ssh stopped', nodeName)
    await target._private.cleanup()
    console.log('node %s instance terminated', nodeName)
    proc.kill('SIGTERM')
  }

  const result: ActyxNode = {
    name: nodeName,
    target,
    ax,
    _private: {
      shutdown,
      actyxBinaryPath: './actyx',
      workingDir,
      axBinaryPath,
      hostname: 'localhost',
      adminPort: port4458,
      swarmPort: 4001,
      apiPort: port4454,
    },
    ...theRest,
  }

  return result
}
