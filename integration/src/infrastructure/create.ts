/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import { createInstance, terminateInstance } from './aws'
import { mkNodeSshDocker, mkNodeSshProcess } from './linux'
import { ActyxOSNode, AwsKey, Target, TargetKind } from './types'
import { Arch, currentArch, currentOS, HostConfig } from '../../jest/types'
import { mkNodeLocalDocker, mkNodeLocalProcess } from './local'
import { LogEntry, MyGlobal } from '../../jest/setup'

const decodeAwsArch = (instance: EC2.Instance): Arch => {
  switch (instance.Architecture) {
    case 'x86_64':
      return 'x86_64'
    case 'arm64':
      return 'aarch64'
    default:
      throw new Error(`unknown AWS arch: ${instance.Architecture}`)
  }
}

const createAwsInstance = async (
  ec2: EC2,
  prepare: { type: 'create-aws-ec2'; ami: string; instance: string; user: string },
  key: AwsKey,
): Promise<Target> => {
  const instance = await createInstance(ec2, {
    InstanceType: prepare.instance,
    ImageId: prepare.ami,
    KeyName: key.keyName,
  })
  const os = instance.Platform === 'Windows' ? 'win' : 'linux'
  const arch = decodeAwsArch(instance)
  const kind: TargetKind = {
    type: 'aws',
    instance: instance.InstanceId!,
    privateAddress: instance.PrivateIpAddress!,
    host: instance.PublicIpAddress!,
    username: prepare.user,
    privateKey: key.privateKey,
  }
  const shutdown = () => terminateInstance(ec2, instance.InstanceId!)
  return { os, arch, kind, _private: { cleanup: shutdown } }
}

const installProcess = async (target: Target, host: HostConfig, logger: (line: string) => void) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh': {
      return await mkNodeSshProcess(host.name, target, kind, logger)
    }
    case 'local': {
      return await mkNodeLocalProcess(host.name, target, logger)
    }
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
    case 'ssh': {
      return await mkNodeSshDocker(host.name, target, kind, logger, gitHash)
    }
    case 'local': {
      return await mkNodeLocalDocker(host.name, target, gitHash, logger)
    }
    default:
      console.error('unknown kind:', kind)
  }
}

/**
 * Create a new
 * @param host
 */
export const createNode = async (host: HostConfig): Promise<ActyxOSNode | undefined> => {
  const { ec2, key, gitHash, thisTestEnvNodes: envNodes } = (<MyGlobal>global).axNodeSetup

  let target: Target | undefined = undefined

  const { prepare } = host
  switch (prepare.type) {
    case 'create-aws-ec2': {
      target = await createAwsInstance(ec2, prepare, key)
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

  if (target === undefined) {
    console.error('no recipe to prepare node %s', host.name)
    return
  }

  const logs: LogEntry[] = []
  const logger = (line: string) => {
    logs.push({ time: new Date(), line })
  }

  try {
    let node: ActyxOSNode | undefined
    switch (host.install) {
      case 'linux':
        node = await installProcess(target, host, logger)
        break
      case 'docker':
        node = await installDocker(target, host, logger, gitHash)
        break
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
        process.stdout.write(`\n****\nlogs for node ${host.name}\n****\n\n`)
        for (const entry of logs) {
          process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
        }
        logs.length = 0
      }
    }

    if (envNodes !== undefined && node !== undefined) {
      envNodes.push(node)
    }

    return node
  } catch (e) {
    console.error('node %s error while setting up:', host.name, e)
    for (const entry of logs) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
    await target._private.cleanup()
  }
}
