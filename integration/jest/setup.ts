import { EC2 } from 'aws-sdk'
import { CLI } from '../src/cli'
import { SettingsInput } from '../src/cli/exec'
import { createKey, deleteKey } from '../src/infrastructure/aws'
import { ActyxOSNode, AwsKey, printTarget } from '../src/infrastructure/types'
import { setupTestProjects } from '../src/setup-projects'
import { promises as fs } from 'fs'
import { Config, Settings } from './types'
import YAML from 'yaml'
import { rightOrThrow } from '../src/infrastructure/rightOrThrow'
import execa from 'execa'
import { createNode } from '../src/infrastructure/create'
import { retryTimes } from '../src/retry'

export type LogEntry = {
  time: Date
  line: string
}

export type NodeSetup = {
  nodes: ActyxOSNode[]
  ec2: EC2
  key: AwsKey
  settings: Settings
  gitHash: string
  thisTestEnvNodes?: ActyxOSNode[]
}

export type MyGlobal = typeof global & { axNodeSetup: NodeSetup }

const getGitHash = async (settings: Settings) => {
  if (settings.gitHash !== null) {
    return settings.gitHash
  }
  const result = await execa.command('git rev-parse HEAD')
  return result.stdout
}

const getPeerId = async (ax: CLI, retries = 10): Promise<string | undefined> => {
  await new Promise((res) => setTimeout(res, 1000))
  const state = await retryTimes(ax.Swarms.State, 3)
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

const getNumPeersMax = async (nodes: ActyxOSNode[]): Promise<number> => {
  const getNumPeersOne = async (ax: CLI) => {
    const state = await retryTimes(ax.Swarms.State, 3)
    if ('Err' in state) {
      console.log(`error getting peers: ${state.Err.message}`)
      return -1
    }
    const numPeers = Object.values(state.Ok.swarm.peers).filter(
      (peer) => peer.connection_state === 'Connected',
    ).length
    return numPeers
  }
  const res = await Promise.all(nodes.map((node) => getNumPeersOne(node.ax)))
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
    const currentPeers = await getNumPeersMax(bootstrap)
    if (currentPeers !== numPeers) {
      console.log('  numPeers = ', currentPeers)
      numPeers = currentPeers
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
const setupInternal = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('\n')

  const configFile = process.env.AX_CI_HOSTS || 'hosts.yaml'
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
    gitHash: await getGitHash(config.settings),
  }
  Object.assign(axNodeSetup, axNodeSetupObject)

  process.on('SIGINT', () => {
    axNodeSetup.nodes.forEach((node) => node._private.shutdown())
    deleteKey(ec2, axNodeSetup.key.keyName)
  })

  /*
   * Create all the nodes as described in the settings.
   */
  for (const node of await Promise.all(
    config.hosts.map((host) =>
      createNode(host).catch(console.error.bind('node %s cannot create AWS node:', host.name)),
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

const setup = async (config: Record<string, unknown>): Promise<void> => {
  const started = process.hrtime.bigint()
  const timer = setInterval(
    () =>
      console.log(
        ' - clock: %i seconds',
        Math.floor(Number((process.hrtime.bigint() - started) / BigInt(1_000_000_000))),
      ),
    10_000,
  )

  try {
    return await setupInternal(config)
  } finally {
    clearInterval(timer)
  }
}

export default setup
