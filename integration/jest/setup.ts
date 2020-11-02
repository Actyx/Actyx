/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import { CLI } from '../src/ax'
import { SettingsInput } from '../src/ax/exec'
import demoMachineKit from '../src/ax/setup-projects/demo-machine-kit'
import quickstart from '../src/ax/setup-projects/quickstart'
import { createInstance, createKey, terminateInstance } from '../src/runner/aws'
import { mkNodeLinux } from '../src/runner/linux'
import { ActyxOSNode, AwsKey } from '../src/runner/types'

type LogEntry = {
  time: Date
  line: string
}

export type NodeSetup = {
  nodes: ActyxOSNode[]
  ec2: EC2
  key: AwsKey
  logs: { [n: string]: LogEntry[] }
  keepNodesRunning: boolean
}

export type MyGlobal = typeof global & { axNodeSetup: NodeSetup }

const createNode = async (
  ec2: EC2,
  key: AwsKey,
  name: string,
): Promise<[LogEntry[], ActyxOSNode] | undefined> => {
  try {
    const instance = await createInstance(ec2, {
      ImageId: 'ami-0254f49f790a514ab', // Debian 11 from Oct 5, 2020
      InstanceType: 't2.small',
      MinCount: 1,
      MaxCount: 1,
      SecurityGroupIds: ['sg-0d942c552d4ff817c'],
      KeyName: key.keyName,
      SubnetId: 'subnet-0f6bd6dc4ce64810e',
      InstanceInitiatedShutdownBehavior: 'terminate',
    })

    const logs: LogEntry[] = []
    const logger = (line: string) => {
      logs.push({ time: new Date(), line })
    }

    try {
      const node = await mkNodeLinux(
        name,
        {
          os: 'linux',
          arch: 'x86_64',
          kind: {
            type: 'aws',
            instance: instance.InstanceId!,
            privateAddress: instance.PrivateIpAddress!,
            host: instance.PublicIpAddress!,
            username: 'admin',
            privateKey: key.privateKey,
          },
          _private: {
            shutdown: () => terminateInstance(ec2, instance.InstanceId!),
          },
        },
        logger,
      )
      return [logs, node]
    } catch (e) {
      console.error('node %s error while setting up:', name, e)
      for (const entry of logs) {
        process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
      }
      await terminateInstance(ec2, instance.InstanceId!)
    }
  } catch (e) {
    console.error('node %s cannot create AWS node:', name, e)
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

const setInitialSettings = async (bootstrap: ActyxOSNode, swarmKey: string): Promise<void> => {
  const result = await bootstrap.ax.Settings.Set(
    'com.actyx.os',
    SettingsInput.FromValue({
      general: {
        swarmKey,
        displayName: 'test',
      },
      services: { eventService: { topic: 'a' } },
    }),
  ).catch(console.error)
  console.log('set settings result:', result)
}

const setAllSettings = async (
  bootstrap: ActyxOSNode & { target: { kind: { type: 'aws' } } },
  peerId: string,
  nodes: ActyxOSNode[],
  swarmKey: string,
): Promise<void> => {
  const ips = [bootstrap.target.kind.host, bootstrap.target.kind.privateAddress]
  const bootstrapNodes = ips.map((ip) => `/ip4/${ip}/tcp/4001/ipfs/${peerId}`)

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

const getPeers = async (node: ActyxOSNode): Promise<number> => {
  const state = await node.ax.Swarms.State()
  if ('Err' in state) {
    console.log(`error getting peers: ${state.Err.message}`)
    return -1
  }
  const numPeers = Object.values(state.Ok.swarm.peers).filter(
    (peer) => peer.connection_state === 'Connected',
  ).length
  console.log(`  numPeers = ${numPeers}`)
  return numPeers
}

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('\n')

  // install sample apps locally for t esting
  try {
    const quickstartStatusMessage = await quickstart.setup()
    console.log(quickstartStatusMessage)
    process.stdout.write('\n')

    const demoMachineKitStatusMessage = await demoMachineKit.setup()
    console.log(demoMachineKitStatusMessage)
    process.stdout.write('\n')
  } catch (err) {
    console.error(err)
    process.exitCode = 1
    process.exit()
  }

  const axNodeSetup = (<MyGlobal>global).axNodeSetup

  process.stdout.write('\n')

  // CRITICAL: must define all NodeSetup fields here to avoid undefined reference errors
  axNodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
  axNodeSetup.key = await createKey(axNodeSetup.ec2)
  axNodeSetup.nodes = []
  axNodeSetup.logs = {}

  process.on('SIGINT', () => axNodeSetup.nodes.forEach((node) => node._private.shutdown()))

  for (const res of await Promise.all(
    ['pool1', 'pool2'].map((name) => createNode(axNodeSetup.ec2, axNodeSetup.key, name)),
  )) {
    if (res === undefined) {
      continue
    }
    const [logs, node] = res
    axNodeSetup.nodes.push(node)
    axNodeSetup.logs[node.name] = logs
  }

  const bootstrap = axNodeSetup.nodes.find(
    (node): node is ActyxOSNode & { target: { kind: { type: 'aws' } } } =>
      node.target.kind.type === 'aws' && node.host === 'process',
  )
  if (bootstrap === undefined) {
    return
  }

  console.log(`setting up bootstrap node ${bootstrap.name}`)

  // need to set some valid settings to be able to get the peerId
  const swarmKey = await bootstrap.ax.Swarms.KeyGen()

  if (swarmKey.code !== 'OK') {
    new Error('cannot generate swarmkey')
    return
  }

  const key = swarmKey.result.swarmKey

  await setInitialSettings(bootstrap, key)

  const peerId = await getPeerId(bootstrap.ax)
  if (peerId === undefined) {
    console.error('timeout waiting for store to start')
    return
  }
  console.log(`bootstrap node ${bootstrap.name} has PeerId ${peerId}`)

  await setAllSettings(bootstrap, peerId, axNodeSetup.nodes, key)

  console.log('bootstrap node set up, settings all set')

  let attempts = 60
  while ((await getPeers(bootstrap)) < axNodeSetup.nodes.length - 1 && attempts-- > 0) {
    await new Promise((res) => setTimeout(res, 1000))
  }
  if (attempts === -1) {
    console.error('swarm did not fully connect')
  } else {
    console.error('swarm fully connected')
  }
}

export default setup
