/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import { CLI } from '../src/cli'
import { SettingsInput } from '../src/cli/exec'
import { createInstance, createKey, deleteKey, terminateInstance } from '../src/infrastructure/aws'
import { mkNodeSshDocker, mkNodeSshProcess } from '../src/infrastructure/linux'
import { ActyxOSNode, AwsKey, printTarget, Target, TargetKind } from '../src/infrastructure/types'
import { setupTestProjects } from '../src/setup-projects'
import { promises as fs } from 'fs'
import { Arch, Config, currentArch, currentOS, HostConfig, Settings } from './types'
import YAML from 'yaml'
import { rightOrThrow } from '../src/infrastructure/rightOrThrow'
import execa from 'execa'
import { mkNodeLocalDocker, mkNodeLocalProcess } from '../src/infrastructure/local'

type LogEntry = {
  time: Date
  line: string
}

export type NodeSetup = {
  nodes: ActyxOSNode[]
  ec2: EC2
  key: AwsKey
  settings: Settings
}

export type MyGlobal = typeof global & { axNodeSetup: NodeSetup }

const getGitHash = async () => {
  const result = await execa('git', ['log', '-1', '--pretty=%H'])
  return result.stdout
}

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
  return { os, arch, kind, _private: { shutdown } }
}

const installProcess = async (target: Target, host: HostConfig, logger: (line: string) => void) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws': {
      return await mkNodeSshProcess(host.name, target, kind, logger)
    }
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
    case 'aws': {
      return await mkNodeSshDocker(host.name, target, kind, logger, gitHash)
    }
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

const createNode = async (
  ec2: EC2,
  key: AwsKey,
  host: HostConfig,
  gitHash: string,
): Promise<ActyxOSNode | undefined> => {
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
        _private: { shutdown },
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

    return node
  } catch (e) {
    console.error('node %s error while setting up:', host.name, e)
    for (const entry of logs) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
    await target._private.shutdown()
  }
}

const getPeerId = async (ax: CLI, retries = 10): Promise<string | undefined> => {
  await new Promise((res) => setTimeout(res, 1000))
  const state = await ax.Swarms.State()
  if ('Err' in state) {
    return retries === 0 ? undefined : getPeerId(ax, retries - 1)
  } else {
    return state.Ok.swarm.peer_id
  }
}

const setInitialSettings = async (bootstrap: ActyxOSNode[], swarmKey: string): Promise<void> => {
  for (const node of bootstrap) {
    const result = await node.ax.Settings.Set(
      'com.actyx.os',
      SettingsInput.FromValue({
        general: {
          swarmKey,
          displayName: 'initial',
        },
        services: { eventService: { topic: 'a' } },
      }),
    ).catch(console.error)
    if (result !== undefined && result.code !== 'OK') {
      console.log('node %s set settings result:', node, result)
    }
  }
}

const getBootstrapNodes = async (bootstrap: ActyxOSNode[]): Promise<string[]> => {
  const ret = []
  for (const { node, pid } of await Promise.all(
    bootstrap.map(async (node) => ({ node, pid: await getPeerId(node.ax) })),
  )) {
    const addr = []
    const kind = node.target.kind
    if ('host' in kind) {
      addr.push(kind.host)
    }
    if (kind.type === 'aws') {
      addr.push(kind.privateAddress)
    }
    if (pid !== undefined) {
      ret.push(...addr.map((a) => `/ip4/${a}/tcp/4001/ipfs/${pid}`))
    }
  }
  return ret
}

const setAllSettings = async (
  bootstrap: (ActyxOSNode & { host: 'process' })[],
  nodes: ActyxOSNode[],
  swarmKey: string,
): Promise<void> => {
  const bootstrapNodes = await getBootstrapNodes(bootstrap)

  const settings = (displayName: string) => ({
    general: {
      bootstrapNodes,
      displayName,
      logLevels: { apps: 'INFO', os: 'DEBUG' },
      swarmKey,
    },
    licensing: { apps: {}, os: 'development' },
    services: {
      eventService: {
        readOnly: false,
        topic: 'Cosmos integration',
      },
    },
  })

  const result = await Promise.all(
    nodes.map((node) =>
      node.ax.Settings.Set('com.actyx.os', SettingsInput.FromValue(settings(node.name))),
    ),
  )
  const errors = result.map((res, idx) => ({ res, idx })).filter(({ res }) => res.code !== 'OK')
  console.log('%i errors', errors.length)
  for (const { res, idx } of errors) {
    console.log('%s:', nodes[idx], res)
  }
}

const getPeers = async (nodes: ActyxOSNode[]): Promise<number> => {
  const getPeersOne = async (ax: CLI) => {
    const state = await ax.Swarms.State()
    if ('Err' in state) {
      console.log(`error getting peers: ${state.Err.message}`)
      return -1
    }
    const numPeers = Object.values(state.Ok.swarm.peers).filter(
      (peer) => peer.connection_state === 'Connected',
    ).length
    return numPeers
  }
  const res = await Promise.all(nodes.map((node) => getPeersOne(node.ax)))
  return res.reduce((a, b) => Math.max(a, b), 0)
}

const configureBoostrap = async (nodes: ActyxOSNode[]) => {
  // All process-hosted nodes can serve as bootstrap nodes
  const bootstrap = nodes.filter(
    (node): node is ActyxOSNode & { host: 'process' } => node.host === 'process',
  )
  if (bootstrap.length === 0) {
    console.error('cannot find suitable bootstrap nodes')
    return
  }

  console.log(`setting up bootstrap nodes ${bootstrap.map((node) => node.name)}`)

  // need to set some valid settings to be able to get the peerId
  const swarmKey = await bootstrap[0].ax.Swarms.KeyGen()
  if (swarmKey.code !== 'OK') {
    new Error('cannot generate swarmkey')
    return
  }
  const key = swarmKey.result.swarmKey
  await setInitialSettings(bootstrap, key)

  // get bootstrap nodes’ peerId and then set the correct settings on all nodes
  await setAllSettings(bootstrap, nodes, key)

  console.log('bootstrap node set up, settings all set')

  // wait for the swarm to connect (precisely: for all nodes to connect to bootstrap)
  let attempts = 60
  let numPeers = 0
  do {
    attempts -= 1
    await new Promise((res) => setTimeout(res, 1000))
    const now = await getPeers(bootstrap)
    if (now !== numPeers) {
      console.log('  numPeers = ', now)
      numPeers = now
    }
  } while (numPeers < nodes.length - 1 && attempts-- > 0)
  if (attempts === -1) {
    console.error('swarm did not fully connect')
  } else {
    console.error('swarm fully connected')
  }
}

/**
 * Create and/or install ActyxOS nodes and wait until they form a swarm.
 * @param _config
 */
const setup = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('\n')

  const configFile = process.env.HOSTS || 'hosts.yaml'
  console.log('Running Jest with hosts described in ' + configFile)

  const configObject = YAML.parse(await fs.readFile(configFile, 'utf-8'))
  const config = rightOrThrow(Config.decode(configObject), configObject)
  console.log('using %i hosts', config.hosts.length)

  const projects = config.settings.skipTestProjectPreparation
    ? Promise.resolve()
    : setupTestProjects(config.settings.tempDir)

  // CRITICAL: axNodeSetup does not yet have all the fields of the NodeSetup type at this point
  // so we get the (partial) object’s reference, construct a fully type-checked NodeSetup, and
  // then make the global.axNodeSetup complete by copying the type-checked properties into it.
  const axNodeSetup = (<MyGlobal>global).axNodeSetup
  const ec2 = new EC2({ region: 'eu-central-1' })
  const axNodeSetupObject: NodeSetup = {
    ec2,
    key: await createKey(ec2),
    nodes: [],
    settings: config.settings,
  }
  Object.assign(axNodeSetup, axNodeSetupObject)

  process.on('SIGINT', () => {
    axNodeSetup.nodes.forEach((node) => node._private.shutdown())
    deleteKey(ec2, axNodeSetup.key.keyName)
  })

  const gitHash = await getGitHash()

  /*
   * Create all the nodes as described in the settings.
   */
  for (const node of await Promise.all(
    config.hosts.map((host) =>
      createNode(axNodeSetup.ec2, axNodeSetup.key, host, gitHash).catch(
        console.error.bind('node %s cannot create AWS node:', host.name),
      ),
    ),
  )) {
    if (node === undefined) {
      continue
    }
    axNodeSetup.nodes.push(node)
  }

  console.log(
    '\n*** ActyxOS nodes started ***\n\n- ' +
      axNodeSetup.nodes
        .map(
          (node) => `${node.name} on ${printTarget(node.target)} with runtimes [${node.runtimes}]`,
        )
        .join('\n- ') +
      '\n',
  )

  console.log('waiting for project setup to finish')
  await projects

  try {
    await configureBoostrap(axNodeSetup.nodes)
  } catch (error) {
    console.log('error while setting up bootstrap:', error)
    await Promise.all(axNodeSetup.nodes.map((node) => node._private.shutdown()))
    throw new Error('configuring bootstrap failed')
  }
}

export default setup
