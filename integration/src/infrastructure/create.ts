/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { ensureDirSync } from 'fs-extra'
import { EC2 } from 'aws-sdk'
import { createInstance, instanceToTarget } from './aws'
import { mkNodeSshDocker, mkNodeSshProcess } from './linux'
import { ActyxOSNode, AwsKey, Target } from './types'
import { CreateEC2, currentArch, currentOS, HostConfig } from '../../jest/types'
import { mkNodeLocalDocker, mkNodeLocalProcess } from './local'
import { LogEntry, MyGlobal } from '../../jest/setup'
import fs from 'fs'
import path from 'path'
import { mkNodeWinRM } from './windows'

const createAwsInstance = async (
  ec2: EC2,
  prepare: CreateEC2,
  key: AwsKey,
  hostname: string,
  runIdentifier: string,
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
  })
  return instanceToTarget(instance, prepare, key, ec2)
}

const installProcess = async (target: Target, host: HostConfig, logger: (line: string) => void) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return await mkNodeSshProcess(host.name, target, kind, logger)

    case 'local':
      return await mkNodeLocalProcess(host.name, target, logger)

    default:
      console.error('unknown kind:', kind)
  }
}

const installDocker = async (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
  gitHash: string,
) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return await mkNodeSshDocker(host.name, target, kind, logger, gitHash)

    case 'local':
      return await mkNodeLocalDocker(host.name, target, gitHash, logger)

    default:
      console.error('unknown kind:', kind)
  }
}

const installWindows = async (
  ec2: EC2,
  prepare: CreateEC2,
  key: AwsKey,
  host: HostConfig,
  ciRun: string,
  publicKeyPath: string,
  logger: (line: string) => void,
) => {
  // create some random string of at least 20 characters
  const adminPW = Math.random().toString(36).substring(2) + Math.random().toString(36).substring(2)
  return await mkNodeWinRM(ec2, prepare, key, ciRun, host.name, adminPW, publicKeyPath, logger)
}

/**
 * Create a new node from the HostConfig that describes it. This can entail spinning up an EC2
 * host or it can mean using locally available resources like a Docker daemon.
 *
 * @param host
 */
export const createNode = async (host: HostConfig): Promise<ActyxOSNode | undefined> => {
  const {
    ec2,
    key,
    gitHash,
    thisTestEnvNodes,
    settings: { logToStdout },
    runIdentifier,
  } = (<MyGlobal>global).axNodeSetup

  let target: Target | undefined = undefined

  if (host.install !== 'windows') {
    const { prepare, name: hostname } = host
    switch (prepare.type) {
      case 'create-aws-ec2': {
        target = await createAwsInstance(ec2, prepare, key, hostname, runIdentifier)
        break
      }
      case 'local': {
        console.log('node %s using the local system', host.name)
        const shutdown = () => Promise.resolve()
        target = {
          os: currentOS(),
          arch: currentArch(),
          _private: { cleanup: shutdown },
          kind: { type: 'local' },
        }
        break
      }
    }
  }

  const logs: LogEntry[] = []
  const logger = (line: string) => {
    logs.push({ time: new Date(), line })
  }

  try {
    let node: ActyxOSNode | undefined
    switch (host.install) {
      case 'linux':
        if (target === undefined) {
          console.error('no recipe to prepare node %s', host.name)
          return
        }
        node = await installProcess(target, host, logger)
        break
      case 'docker':
        if (target === undefined) {
          console.error('no recipe to prepare node %s', host.name)
          return
        }
        node = await installDocker(target, host, logger, gitHash)
        break
      case 'windows': {
        const { prepare } = host
        if (prepare.type !== 'create-aws-ec2') {
          console.error('can only install windows on EC2, not', prepare)
          return
        }
        node = await installWindows(
          ec2,
          prepare,
          key,
          host,
          runIdentifier,
          key.publicKeyPath,
          logger,
        )
        break
      }
      default:
        return
    }

    if (node === undefined) {
      console.error('no recipe to install node %s', host.name)
    } else {
      const shutdown = node._private.shutdown
      node._private.shutdown = async () => {
        await shutdown().catch((error) =>
          console.error('node %s error while shutting down:', host.name, error),
        )
        const logFilePath = mkLogFilePath(runIdentifier, host)
        const [logSink, flush] = logToStdout
          ? [process.stdout.write, () => ({})]
          : appendToFile(logFilePath)

        process.stdout.write(
          `\n****\nlogs for node ${host.name}${
            logToStdout ? '' : ` redirected to "${logFilePath}"`
          }\n****\n\n`,
        )
        for (const entry of logs) {
          logSink(`${entry.time.toISOString()} ${entry.line}\n`)
        }
        flush()
        logs.length = 0
      }
    }

    if (thisTestEnvNodes !== undefined && node !== undefined) {
      thisTestEnvNodes.push(node)
    }

    return node
  } catch (e) {
    console.error('node %s error while setting up:', host.name, e)
    for (const entry of logs) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
    await target?._private.cleanup()
    throw e
  }
}

// Constructs a log file path for a given `runId` and a `host`. Will create any
// needed folders.
const mkLogFilePath = (runId: string, host: HostConfig) => {
  const folder = path.resolve('logs', runId)
  ensureDirSync(folder)
  return path.resolve(folder, host.name)
}

const appendToFile = (fileName: string): [(_: string) => void, () => void] => {
  const fd = fs.openSync(fileName, 'a')
  return [(line: string) => fs.writeSync(fd, line), () => fs.closeSync(fd)]
}
