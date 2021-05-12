/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { ensureDirSync } from 'fs-extra'
import { EC2 } from 'aws-sdk'
import { createInstance, instanceToTarget } from './aws'
import { mkNodeSshDocker, mkNodeSshProcess } from './linux'
import { ActyxNode, AwsKey, SshAble, Target, TargetKind } from './types'
import { CreateEC2, currentArch, currentOS, HostConfig, UseSsh } from '../../jest/types'
import { mkNodeLocalDocker, mkNodeLocalProcess } from './local'
import { LogEntry, MyGlobal } from '../../jest/setup'
import fs, { readFileSync } from 'fs'
import path from 'path'
import { makeWindowsInstallScript, mkWindowsSsh } from './windows'
import { mkExecute } from '.'
import { mkNodeSshAndroid } from './android'

const createAwsInstance = async (
  ec2: EC2,
  prepare: CreateEC2,
  key: AwsKey,
  hostname: string,
  runIdentifier: string,
  userData?: string,
): Promise<Target> => {
  const instance = await createInstance(ec2, {
    InstanceType: prepare.instance,
    ImageId: prepare.ami,
    KeyName: key.keyName,
    TagSpecifications: [
      {
        ResourceType: 'instance',
        Tags: [
          { Key: 'Name', Value: hostname },
          { Key: 'ci_run', Value: runIdentifier },
        ],
      },
    ],
    UserData: userData,
  })
  return instanceToTarget(instance, prepare, key, ec2)
}

const installProcess = (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
): Promise<ActyxNode> => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      return host.install.type === 'windows'
        ? mkWindowsSsh(host.name, target, kind, logger)
        : mkNodeSshProcess(host.name, target, kind, logger)
    }
    case 'local':
      return mkNodeLocalProcess(host.name, target)(logger)
    case 'test':
      throw new Error(`${kind.type} is not supported as proc`)
  }
}

const installDocker = (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
  gitHash: string,
): Promise<ActyxNode> => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return mkNodeSshDocker(host.name, target, kind, logger, gitHash)
    case 'local':
      return mkNodeLocalDocker(host.name, target, gitHash, logger)
    case 'test':
      throw new Error(`${kind.type} is not supported as docker proc`)
  }
}

const installAndroidEmulator = (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
): Promise<ActyxNode> => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return mkNodeSshAndroid(host.name, target, kind, logger)
    case 'local':
    case 'test': {
      throw new Error(`Unsupported kind: ${kind}`)
    }
  }
}

export const mkLocalTarget = (hostname: string, reuseWorkingDirIfExists?: boolean): Target => {
  console.log('node %s using the local system', hostname)
  const shutdown = () => Promise.resolve()
  const os = currentOS()
  const kind: TargetKind = { type: 'local', reuseWorkingDirIfExists }
  const execute = mkExecute(os, kind)

  return {
    os,
    arch: currentArch(),
    execute,
    _private: {
      cleanup: shutdown,
    },
    kind,
  }
}

const mkAwsEc2 = (
  host: HostConfig,
  prepare: CreateEC2,
  runIdentifier: string,
  ec2?: EC2,
  key?: AwsKey,
): Promise<Target> => {
  if (typeof key === 'undefined' || typeof ec2 === 'undefined') {
    throw 'No AWS EC2 Keypair was created. Are you authenticated with AWS?'
  }
  if (host.install.type === 'windows') {
    const pubKey = readFileSync(key.publicKeyPath)
    const enableSshScript = makeWindowsInstallScript(pubKey.toString('utf8'))
    const userData = Buffer.from(enableSshScript).toString('base64')
    return createAwsInstance(ec2, prepare, key, host.name, runIdentifier, userData)
  } else {
    return createAwsInstance(ec2, prepare, key, host.name, runIdentifier)
  }
}

const mkSshTarget = (prepare: UseSsh): Target => {
  const { os, arch, user, privateKeyPath, host } = prepare
  const sshable: SshAble = {
    host,
    privateKey: privateKeyPath,
    username: user,
  }
  const kind: TargetKind = { type: 'ssh', ...sshable }
  const execute = mkExecute(os, kind)
  return {
    arch,
    os,
    execute,
    kind,
    _private: { cleanup: () => Promise.resolve() },
  }
}

const mkTarget = (
  host: HostConfig,
  runIdentifier: string,
  ec2?: EC2,
  key?: AwsKey,
): Promise<Target> => {
  const { prepare } = host
  switch (prepare.type) {
    case 'create-aws-ec2': {
      return mkAwsEc2(host, prepare, runIdentifier, ec2, key)
    }
    case 'ssh': {
      return Promise.resolve(mkSshTarget(prepare))
    }
    case 'local': {
      return Promise.resolve(mkLocalTarget(host.name))
    }
  }
}

const mkActyxNode = (host: HostConfig, target: Target, gitHash: string) => (
  logger: (line: string) => void,
): Promise<ActyxNode> => {
  const { install } = host
  switch (install.type) {
    case 'linux':
    case 'windows': {
      return installProcess(target, host, logger)
    }
    case 'docker': {
      return installDocker(target, host, logger, gitHash)
    }
    case 'android': {
      return installAndroidEmulator(target, host, logger)
    }

    case 'just-use-a-running-actyx-node': {
      throw new Error(`Not implemented ${install.type}`)
    }
  }
}

const logEntryToStr = (x: LogEntry): string => `${x.time.toISOString()} ${x.line}\n`

export const mkActyxNodeWithLogging = async (
  runIdentifier: string,
  logToStdout: boolean,
  nodeName: string,
  mkNode: (logger: (line: string) => void) => Promise<ActyxNode>,
): Promise<ActyxNode> => {
  const logs: LogEntry[] = []
  const logger = (line: string) => {
    logs.push({ time: new Date(), line })
  }

  try {
    const node0 = await mkNode(logger)

    const orig_shutdown = node0._private.shutdown
    const shutdown = async () => {
      await orig_shutdown().catch((error) =>
        console.error('node %s error while shutting down:', nodeName, error),
      )
      const logFilePath = mkLogFilePath(runIdentifier, nodeName)
      const [logSink, flush] = logToStdout
        ? [process.stdout.write, () => ({})]
        : appendToFile(logFilePath)

      process.stdout.write(
        `\n****\nlogs for node ${nodeName}${
          logToStdout ? '' : ` redirected to "${logFilePath}"`
        }\n****\n\n`,
      )
      logs.forEach((x) => logSink(logEntryToStr(x)))
      flush()
      logs.length = 0
    }

    const node = { ...node0, _private: { ...node0._private, shutdown } }

    return node
  } catch (e) {
    console.error('node %s error while setting up:', nodeName, e)
    logs.forEach((x) => process.stdout.write(logEntryToStr(x)))
    throw e
  }
}

/**
 * Create a new node from the HostConfig that describes it. This can entail spinning up an EC2
 * host or it can mean using locally available resources like a Docker daemon.
 *
 * @param host
 */
export const createNode = async (host: HostConfig): Promise<ActyxNode | undefined> => {
  const {
    ec2,
    key,
    gitHash,
    thisTestEnvNodes,
    settings: { logToStdout },
    runIdentifier,
  } = (<MyGlobal>global).axNodeSetup
  const target = await mkTarget(host, runIdentifier, ec2, key)
  try {
    const node = await mkActyxNodeWithLogging(
      runIdentifier,
      logToStdout,
      host.name,
      mkActyxNode(host, target, gitHash),
    )

    if (thisTestEnvNodes !== undefined) {
      thisTestEnvNodes.push(node)
    }

    return node
  } catch (e) {
    await target?._private.cleanup()
    throw e
  }
}

// Constructs a log file path for a given `runId` and a `host`. Will create any
// needed folders.
const mkLogFilePath = (runId: string, filename: string) => {
  const folder = path.resolve('logs', runId)
  ensureDirSync(folder)
  return path.resolve(folder, filename)
}

const appendToFile = (fileName: string): [(_: string) => void, () => void] => {
  const fd = fs.openSync(fileName, 'a')
  return [(line: string) => fs.writeSync(fd, line), () => fs.closeSync(fd)]
}
