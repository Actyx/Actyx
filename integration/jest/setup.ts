/**
 * Responsibilities:
 *   - Read hosts.yaml
 *   - Create nodes as separate ec2 instances or locally
 *   - Bootstrap all nodes in the same swarm
 */
import { EC2 } from 'aws-sdk'
import execa from 'execa'
import { promises as fs } from 'fs'
import YAML from 'yaml'
import { CLI } from '../src/cli'
import { SettingsInput } from '../src/cli/exec'
import { createKey, deleteKey } from '../src/infrastructure/aws'
import { createNode } from '../src/infrastructure/create'
import { rightOrThrow } from '../src/infrastructure/rightOrThrow'
import { ActyxNode, AwsKey, printTarget } from '../src/infrastructure/types'
import { retryTimes } from '../src/retry'
import { Config, Settings } from './types'

export type LogEntry = {
  time: Date
  line: string
}

export type NodeSetup = {
  nodes: ActyxNode[]
  ec2?: EC2
  key?: AwsKey
  settings: Settings
  gitHash: string
  // Unique identifier for this particular run. This is used to group all logs
  // files related to one run (a run being test suites sharing a common global
  // setup/teardown).
  runIdentifier: string
  thisTestEnvNodes?: ActyxNode[]
}

export type MyGlobal = typeof global & { axNodeSetup: NodeSetup }

const currentHead = () => execa.command('git rev-parse HEAD').then((x) => x.stdout)

const getGitHash = async (settings: Settings) => {
  const maybeEnv = process.env['AX_GIT_HASH']
  if (maybeEnv !== undefined && maybeEnv !== null && maybeEnv.length > 0) {
    console.log('Using git hash from environment:', maybeEnv)
    return maybeEnv
  }
  if (settings.gitHash !== null) {
    console.log('Using git hash from settings:', settings.gitHash)
    return settings.gitHash
  }
  const result = await currentHead()
  console.log('Using git hash from current HEAD:', result)
  return result
}

// TODO: Do we need a peer id or we can just specify the ip for the bootstrap node
const getPeerId = async (ax: CLI, retries = 10): Promise<string | undefined> => {
  await new Promise((res) => setTimeout(res, 1000))
  const state = await retryTimes(ax.swarms.state, 3)
  if (state.code != 'OK') {
    return retries === 0 ? undefined : getPeerId(ax, retries - 1)
  } else {
    return state.result.peer_id
  }
}

const setInitialSettings = async (bootstrap: ActyxNode[], swarmKey: string): Promise<void> => {
  for (const node of bootstrap) {
    const result = await node.ax.settings
      .set(
        'com.actyx',
        SettingsInput.FromValue({
          admin: {
            displayName: 'initial',
          },
          swarm: {
            swarmKey,
            topic: 'a',
          },
        }),
      )
      .catch(console.error)
    if (result !== undefined && result.code !== 'OK') {
      console.log('node %s set settings result:', node, result)
    }
  }
}

const getBootstrapNodes = async (bootstrap: ActyxNode[]): Promise<string[]> => {
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
      ret.push(...addr.map((a) => `/ip4/${a}/tcp/${node._private.apiSwarmPort}/p2p/${pid}`))
    }
  }
  return ret
}

const setAllSettings = async (
  bootstrap: (ActyxNode & { host: 'process' })[],
  nodes: ActyxNode[],
  swarmKey: string,
): Promise<void> => {
  const bootstrapNodes = await getBootstrapNodes(bootstrap)

  const settings = (displayName: string) => ({
    admin: {
      displayName,
      logLevels: { node: 'DEBUG' },
    },
    swarm: {
      bootstrapNodes,
      swarmKey,
      topic: 'Cosmos integration',
    },
    licensing: { apps: {}, node: 'development' },
    api: {
      events: {
        readOnly: false,
      },
    },
  })

  const result = await Promise.all(
    nodes.map(async (node) => {
      let retry_cnt = 0
      let result = { code: 'ERR_NODE_UNREACHABLE' }
      while (result.code !== 'OK' && retry_cnt < 10) {
        await new Promise((res) => setTimeout(res, 1000))
        result = await node.ax.settings.set(
          'com.actyx',
          SettingsInput.FromValue(settings(node.name)),
        )
        if (result.code === 'OK') {
          // authorizedKeys were overwritten to be empty, so add this cli's
          // public key one more time
          await node.ax.nodes.ls()
        }

        retry_cnt += 1
      }
      return result
    }),
  )
  const errors = result.map((res, idx) => ({ res, idx })).filter(({ res }) => res.code !== 'OK')
  console.log('%i errors setting settings', errors.length)
  for (const { res, idx } of errors) {
    console.log('%s:', nodes[idx], res)
  }
}

const getNumPeersMax = async (nodes: ActyxNode[]): Promise<number> => {
  const getNumPeersOne = async (ax: CLI) => {
    const state = await retryTimes(ax.swarms.state, 3)
    if (state.code != 'OK') {
      console.log(`error getting peers: ${state.message}`)
      return -1
    }
    const numPeers = state.result.peers.length
    return numPeers
  }
  const res = await Promise.all(nodes.map((node) => getNumPeersOne(node.ax)))
  return res.reduce((a, b) => Math.max(a, b), 0)
}

const configureBootstrap = async (nodes: ActyxNode[]) => {
  // All process-hosted nodes can serve as bootstrap nodes
  const bootstrap = nodes.filter(
    (node): node is ActyxNode & { host: 'process' } => node.host === 'process',
  )
  if (bootstrap.length === 0) {
    throw new Error('cannot find suitable bootstrap nodes')
  }

  console.log(`setting up bootstrap nodes ${bootstrap.map((node) => node.name)}`)

  // need to set some valid settings to be able to get the peerId
  const swarmKey = await bootstrap[0].ax.swarms.keyGen()
  if (swarmKey.code !== 'OK') {
    throw new Error('cannot generate swarmkey')
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
    console.log('swarm fully connected')
  }
}

/**
 * Create and/or install Actyx nodes and wait until they form a swarm.
 * @param _config
 */
const setupInternal = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('\n')

  const configFile = process.env.AX_CI_HOSTS || 'hosts.yaml'
  console.log('Running Jest with hosts described in ' + configFile)

  const configObject = YAML.parse(await fs.readFile(configFile, 'utf-8'))
  const config = rightOrThrow(Config.decode(configObject), configObject)
  console.log('using %i hosts', config.hosts.length)

  // CRITICAL: axNodeSetup does not yet have all the fields of the NodeSetup type at this point
  // so we get the (partial) object’s reference, construct a fully type-checked NodeSetup, and
  // then make the global.axNodeSetup complete by copying the type-checked properties into it.
  const axNodeSetup = (<MyGlobal>global).axNodeSetup
  // Overwrite config from env vars
  const keepNodesRunning = config.settings.keepNodesRunning || process.env['AX_DEBUG'] !== undefined
  const gitHash = await getGitHash(config.settings)
  try {
    await fs.mkdir(config.settings.tempDir)
  } catch (e) {
    //ignore
  }
  let ec2: EC2 | undefined = undefined
  let key: AwsKey | undefined = undefined
  let runIdentifier = 'local-run'
  try {
    ec2 = new EC2({ region: 'eu-central-1' })
    key = await createKey(config, ec2)
    runIdentifier = key.keyName
  } catch (e) {
    console.error('skipping aws setup due to ' + e)
  }
  const axNodeSetupObject: NodeSetup = {
    ec2,
    key,
    nodes: [],
    settings: {
      ...config.settings,
      keepNodesRunning,
      // Only override gitHash in settings if it’s different from the current
      // HEAD. If it's the current HEAD, we signal it by setting it to null. This
      // effectively makes sure, that instead of downloading the artifacts,
      // they're going to be looked up in `Cosmos/dist`.
      gitHash: gitHash === (await currentHead()) ? null : gitHash,
    },
    gitHash,
    runIdentifier,
  }
  Object.assign(axNodeSetup, axNodeSetupObject)

  process.on('SIGINT', () => {
    axNodeSetup.nodes.forEach((node) => node._private.shutdown())
    if (typeof axNodeSetup.key !== 'undefined' && typeof ec2 !== 'undefined') {
      deleteKey(ec2, axNodeSetup.key.keyName)
    }
  })

  /*
   * Create all the nodes as described in the settings.
   */
  try {
    for (const node of await Promise.all(config.hosts.map(createNode))) {
      if (node === undefined) {
        continue
      }
      axNodeSetup.nodes.push(node)
    }
  } catch (e) {
    console.log('Error during node creation (might have been logged already):\n', e)
    console.log('Shutting down ...')
    await Promise.all(axNodeSetup.nodes.map((node) => node._private.shutdown()))
    throw new Error('node creation failed')
  }

  console.log(
    '\n*** Actyx nodes started ***\n\n- ' +
      axNodeSetup.nodes.map((node) => `${node.name} on ${printTarget(node.target)}`).join('\n- ') +
      '\n',
  )

  console.log('waiting for project setup to finish')

  try {
    await configureBootstrap(axNodeSetup.nodes)
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
