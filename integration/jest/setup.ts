/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import { CLI } from '../src/ax'
import { SettingsInput } from '../src/ax/exec'
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
}

export type MyGlobal = typeof global & { nodeSetup: NodeSetup }

const createNode = async (nodeSetup: NodeSetup, name: string): Promise<void> => {
  try {
    const instance = await createInstance(nodeSetup.ec2, {
      ImageId: 'ami-0718a1ae90971ce4d',
      MinCount: 1,
      MaxCount: 1,
      SecurityGroupIds: ['sg-064dfecc275620375'],
      KeyName: nodeSetup.key.keyName,
    })

    const logs: LogEntry[] = []
    nodeSetup.logs[name] = logs
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
            username: 'ubuntu',
            privateKey: nodeSetup.key.privateKey,
          },
          shutdown: () => terminateInstance(nodeSetup.ec2, instance.InstanceId!),
        },
        logger,
      )
      nodeSetup.nodes.push(node)
    } catch (e) {
      console.error('node %s error while setting up:', name, e)
      await terminateInstance(nodeSetup.ec2, instance.InstanceId!)
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

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  const nodeSetup = (<MyGlobal>global).nodeSetup

  process.stdout.write('\n')

  // CRITICAL: must define all NodeSetup fields here to avoid undefined reference errors
  nodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
  nodeSetup.key = await createKey(nodeSetup.ec2)
  nodeSetup.nodes = []
  nodeSetup.logs = {}

  await Promise.all(['pool1', 'pool2'].map((name) => createNode(nodeSetup, name)))

  const bootstrap = nodeSetup.nodes.find(
    (node): node is ActyxOSNode & { target: { kind: { type: 'aws' } } } =>
      node.target.kind.type === 'aws' && node.host === 'process',
  )
  if (bootstrap === undefined) {
    return
  }

  console.log(`setting up bootstrap node ${bootstrap.name}`)

  // need to set some valid settings to get started (that swarm key is no actually used one)
  await bootstrap.ax.Settings.Set(
    'com.actyx.os',
    SettingsInput.FromValue({
      general: {
        bootstrapNodes: [
          '/ip4/3.121.252.117/tcp/4001/ipfs/QmaWM8pMoMYkJrdbUZkxHyUavH3tCxRdCC9NYCnXRfQ4Eg',
        ],
        swarmKey:
          'L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjM3NjQ3NTMzNTgzMTY1NWE3MzQxMzE1NzQ1NjI0MzQ2NDI3OTY5Nzk2MTY1MzgzODcyNTI3ODRkMzY2ZjZmNjQ=',
        displayName: 'test',
      },
      licensing: { apps: {}, os: 'development' },
      services: { eventService: { topic: 'a' } },
    }),
  ).catch(console.error)

  const state = await bootstrap.ax.Swarms.State()
  if ('Err' in state) {
    console.log(state.Err)
    return
  }
  const peerId = await getPeerId(bootstrap.ax)
  if (peerId === undefined) {
    console.error('timeout waiting for store to start')
    return
  }
  console.log(`bootstrap node ${bootstrap.name} has PeerId ${peerId}`)
  const ips = [bootstrap.target.kind.host, bootstrap.target.kind.privateAddress]
  const bootstrapNodes = ips.map((ip) => `/ip4/${ip}/tcp/4001/ipfs/${peerId}`)

  const swarmKey = await bootstrap.ax.Swarms.KeyGen()

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

  await Promise.all(
    nodeSetup.nodes.map((node) =>
      node.ax.Settings.Set('com.actyx.os', SettingsInput.FromValue(settings(node.name))),
    ),
  )

  console.log('bootstrap node set up, settings all set')
}

export default setup
