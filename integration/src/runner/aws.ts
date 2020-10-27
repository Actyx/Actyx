import { EC2 } from 'aws-sdk'
import { MyGlobal } from '../../jest/setup'
import { AwsKey } from './types'

// determines frequency of polling AWS APIs (e.g. waiting for instance start)
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

export const myKey = (<MyGlobal>global).axNodeSetup.key

export const createKey = async (ec2: EC2): Promise<AwsKey> => {
  const keyName = `cosmos-${Date.now()}`
  const { KeyMaterial } = await ec2
    .createKeyPair({
      KeyName: keyName,
      TagSpecifications: [
        { ResourceType: 'key-pair', Tags: [{ Key: 'Customer', Value: 'Cosmos integration' }] },
      ],
    })
    .promise()
  if (KeyMaterial === undefined) {
    throw new Error('cannot create key pair')
  }
  console.log('created key %s', keyName)
  return { keyName, privateKey: KeyMaterial }
}

export const deleteKey = async (ec2: EC2, KeyName: string): Promise<void> => {
  console.log('deleting key pair %s', KeyName)
  await ec2.deleteKeyPair({ KeyName }).promise()
}

export const createInstance = async (
  ec2: EC2,
  params: EC2.RunInstancesRequest,
): Promise<EC2.Instance> => {
  const ts = params.TagSpecifications
  const myTags: EC2.TagSpecification = {
    ResourceType: 'instance',
    Tags: [{ Key: 'Customer', Value: 'Cosmos integration' }],
  }
  const withTags = { ...params, TagSpecifications: ts ? [...ts, myTags] : [myTags] }

  let instance = (await ec2.runInstances(withTags).promise()).Instances?.[0]

  if (instance === undefined) {
    console.error('cannot start instance')
    throw new Error('cannot start instance')
  }
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const id = instance.InstanceId!
  console.log('instance %s created', id)

  while (instance !== undefined && instance.State?.Name === 'pending') {
    const update = await pollDelay(() => ec2.describeInstances({ InstanceIds: [id] }).promise())
    instance = update.Reservations?.[0].Instances?.[0]
  }

  if (instance === undefined || instance.State?.Name !== 'running') {
    console.error('instance %s did not start', id)
    throw new Error('instance did not start, last state was' + instance?.State?.Name)
  }
  console.log('instance %s started', id)

  return instance
}

// prune instances started more than a day ago
const PRUNE_AGE_MS = 86_400_000

export const cleanUpInstances = async (ec2: EC2): Promise<void> => {
  const old = (
    await ec2
      .describeInstances({
        Filters: [
          { Name: 'tag:Customer', Values: ['Cosmos integration'] },
          { Name: 'instance-state-name', Values: ['pending', 'running', 'stopping', 'stopped'] },
        ],
      })
      .promise()
  )?.Reservations
  if (old !== undefined && old.length > 0) {
    const ids = old.flatMap((reservation) =>
      (reservation.Instances || []).flatMap((instance) =>
        instance.InstanceId !== undefined &&
        instance.LaunchTime !== undefined &&
        instance.LaunchTime.getTime() < Date.now() - PRUNE_AGE_MS
          ? [instance.InstanceId]
          : [],
      ),
    )
    console.log('terminating instances', ids)
    await ec2.terminateInstances({ InstanceIds: ids }).promise()
  }
}

export const terminateInstance = async (ec2: EC2, instance: string): Promise<void> => {
  console.log('terminating instance %s', instance)
  await ec2.terminateInstances({ InstanceIds: [instance] }).promise()
}
