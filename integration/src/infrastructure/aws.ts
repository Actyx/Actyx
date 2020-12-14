/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { EC2 } from 'aws-sdk'
import execa from 'execa'
import { promises as fs, createWriteStream } from 'fs'
import { remove } from 'fs-extra'
import path from 'path'
import { MyGlobal } from '../../jest/setup'
import { Arch, Config, CreateEC2 } from '../../jest/types'
import { AwsKey, SshAble, Target, TargetKind } from './types'

// determines frequency of polling AWS APIs (e.g. waiting for instance start)
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

export const myKey = (<MyGlobal>global)?.axNodeSetup?.key

export const createKey = async (config: Config, ec2: EC2): Promise<AwsKey> => {
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

  // obtain public key; this requires writing private key to a file because ssh-keygen says so
  const privateKeyPath = path.resolve(config.settings.tempDir, 'sshPrivateKey')
  await remove(privateKeyPath)
  await fs.writeFile(privateKeyPath, KeyMaterial, {
    mode: 0o400,
  })
  const publicKeyPath = path.resolve(config.settings.tempDir, 'sshPublicKey')
  await remove(publicKeyPath)
  await execa('ssh-keygen', ['-yf', privateKeyPath], {
    stdout: await new Promise((res, rej) => {
      const s = createWriteStream(publicKeyPath)
      s.on('open', () => res(s))
      s.on('error', rej)
      s.on('close', () => console.log('stream closed'))
    }),
  })

  return { keyName, privateKey: KeyMaterial, publicKeyPath }
}

export const deleteKey = async (ec2: EC2, KeyName: string): Promise<void> => {
  console.log('deleting key pair %s', KeyName)
  await ec2.deleteKeyPair({ KeyName }).promise()
}

const DEFAULT_PARAMS: EC2.RunInstancesRequest = {
  MinCount: 1,
  MaxCount: 1,
  SecurityGroupIds: ['sg-0d942c552d4ff817c'],
  SubnetId: 'subnet-09149eb0bab11908d',
  InstanceInitiatedShutdownBehavior: 'terminate',
}

export const createInstance = async (
  ec2: EC2,
  params: Partial<EC2.RunInstancesRequest>,
): Promise<EC2.Instance> => {
  const ts = params.TagSpecifications

  // need to extract the 'instance' tags because each resource type can only be named once
  let instanceTags: EC2.TagList = []
  const instanceTagsIdx = ts?.findIndex((spec) => spec.ResourceType === 'instance')
  if (ts !== undefined && instanceTagsIdx !== undefined && instanceTagsIdx >= 0) {
    instanceTags = ts.splice(instanceTagsIdx, 1)[0].Tags || []
  }

  const myTags: EC2.TagSpecification = {
    ResourceType: 'instance',
    Tags: [...instanceTags, { Key: 'Customer', Value: 'Cosmos integration' }],
  }
  const withTags = {
    ...DEFAULT_PARAMS,
    ...params,
    TagSpecifications: ts ? [...ts, myTags] : [myTags],
  }

  // this is the main thing
  console.log('creating instance', withTags)
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

const decodeAwsArch = (instance: EC2.Instance, armv7: boolean): Arch => {
  switch (instance.Architecture) {
    case 'x86_64':
      return 'x86_64'
    case 'arm64':
      return armv7 ? 'armv7' : 'aarch64'
    default:
      throw new Error(`unknown AWS arch: ${instance.Architecture}`)
  }
}

export const instanceToTarget = (
  instance: EC2.Instance,
  prepare: CreateEC2,
  key: AwsKey,
  ec2: EC2,
): Target & { kind: SshAble } => {
  const os = instance.Platform === 'windows' ? 'windows' : 'linux'
  const arch = decodeAwsArch(instance, prepare.armv7)
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

export const cleanUpInstances = async (ec2: EC2, cutoff: number): Promise<void> => {
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

  if (old === undefined || old.length === 0) {
    console.error('No Cosmos integration instances found')
    return
  }

  const idList = old.flatMap((reservation) =>
    (reservation.Instances || []).flatMap((instance) =>
      instance.InstanceId !== undefined &&
      instance.LaunchTime !== undefined &&
      instance.LaunchTime.getTime() < cutoff
        ? [instance.InstanceId]
        : [],
    ),
  )
  if (idList.length === 0) {
    console.error(
      `No Cosmos integration instances found that were started before ${new Date(
        cutoff,
      ).toISOString()}`,
    )
    return
  }

  console.error('Terminating instances', idList)
  await ec2.terminateInstances({ InstanceIds: idList }).promise()
}

export const cleanUpKeys = async (ec2: EC2, cutoff: number): Promise<void> => {
  const keyPairs = (
    await ec2
      .describeKeyPairs({ Filters: [{ Name: 'tag:Customer', Values: ['Cosmos integration'] }] })
      .promise()
  )?.KeyPairs

  if (keyPairs === undefined || keyPairs.length === 0) {
    console.error('No Cosmos KeyPairs found')
    return
  }

  const names = keyPairs?.flatMap(({ KeyName: name }) => {
    if (name === undefined) {
      return []
    }

    const tsStr = name.split('-')[1]
    if (tsStr === undefined) {
      return []
    }

    // parseInt will return NaN if the parameter is not numeric
    const ts = parseInt(tsStr)

    return ts < cutoff ? [name] : []
  })

  if (names.length === 0) {
    console.error(
      `No Cosmos integration KeyPairs found that were created before ${new Date(
        cutoff,
      ).toISOString()}`,
    )
    return
  }

  for (const n of names) {
    console.error(`Deleting KeyPair: ${n}`)
    await ec2.deleteKeyPair({ KeyName: n }).promise()
  }
}

export const terminateInstance = async (ec2: EC2, instance: string): Promise<void> => {
  console.log('terminating instance %s', instance)
  await ec2.terminateInstances({ InstanceIds: [instance] }).promise()
}
