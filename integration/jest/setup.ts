/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import { createInstance, createKey, terminateInstance } from '../src/runner/aws'
import { mkNodeLinux } from '../src/runner/linux'
import { ActyxOSNode, AwsKey } from '../src/runner/types'

export type NodeSetup = {
  nodes?: ActyxOSNode[]
  ec2?: EC2
  key?: AwsKey
}

export type MyGlobal = typeof global & { nodeSetup: NodeSetup }

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  const nodeSetup = (<MyGlobal>global).nodeSetup

  console.log(process.cwd())

  const ec2 = new EC2({ region: 'eu-central-1' })

  nodeSetup.key = await createKey(ec2)

  const instance = await createInstance(ec2, {
    ImageId: 'ami-0718a1ae90971ce4d',
    MinCount: 1,
    MaxCount: 1,
    SecurityGroupIds: ['sg-064dfecc275620375'],
    KeyName: nodeSetup.key.keyName,
  })

  const node = await mkNodeLinux('pool1', {
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
  })

  nodeSetup.ec2 = ec2
  nodeSetup.nodes = [node]
}

export default setup
