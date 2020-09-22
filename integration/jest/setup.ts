/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
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

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  const nodeSetup = (<MyGlobal>global).nodeSetup

  process.stdout.write('\n')

  const ec2 = new EC2({ region: 'eu-central-1' })

  nodeSetup.ec2 = ec2
  nodeSetup.key = await createKey(ec2)
  nodeSetup.nodes = []
  nodeSetup.logs = {}

  for (const name of ['pool1']) {
    try {
      const instance = await createInstance(ec2, {
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
              host: instance.PublicDnsName!,
              username: 'ubuntu',
              privateKey: nodeSetup.key.privateKey,
            },
            shutdown: () => terminateInstance(ec2, instance.InstanceId!),
          },
          logger,
        )
        nodeSetup.nodes.push(node)
      } catch (e) {
        console.error('node %s error while setting up:', name, e)
        terminateInstance(ec2, instance.InstanceId!)
      }
    } catch (e) {
      console.error('node %s cannot create AWS node:', name, e)
    }
  }
}

export default setup
