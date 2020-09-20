import AWS from 'aws-sdk'
import { readFileSync } from 'fs'
import * as Ssh from './ssh'

const sshKey = {
  keyName: 'rkuhn-ec2',
  privateKey: readFileSync('/Users/rkuhn/.ssh/rkuhn-ec2.pem'),
}

// determines frequency of polling AWS APIs (e.g. waiting for instance start)
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 5000)).then(f)

export const mkInstance = async (): Promise<void> => {
  const ec2 = new AWS.EC2({ region: 'eu-central-1' })

  // clean up first
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
        instance.InstanceId !== undefined ? [instance.InstanceId] : [],
      ),
    )
    console.log('terminating instances', ids)
    await ec2.terminateInstances({ InstanceIds: ids }).promise()
  }

  let instance = (
    await ec2
      .runInstances({
        ImageId: 'ami-0718a1ae90971ce4d',
        MinCount: 1,
        MaxCount: 1,
        SecurityGroupIds: ['sg-064dfecc275620375'],
        SubnetId: '',
        TagSpecifications: [
          { ResourceType: 'instance', Tags: [{ Key: 'Customer', Value: 'Cosmos integration' }] },
        ],
        KeyName: sshKey.keyName,
      })
      .promise()
  ).Instances?.[0]
  if (instance === undefined) {
    console.error('cannot start instance')
    return
  }
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const id = instance.InstanceId!
  console.log('instance %s created', id)

  process.stdout.write('waiting for instance start ')
  while (instance !== undefined && instance.State?.Name === 'pending') {
    const update = await pollDelay(() => ec2.describeInstances({ InstanceIds: [id] }).promise())
    process.stdout.write('.')
    instance = update.Reservations?.[0].Instances?.[0]
  }
  process.stdout.write('\n')

  if (instance === undefined || instance.State?.Name !== 'running') {
    console.error('instance did not start')
    return
  }
  console.log('instance started', instance)

  const ssh = new Ssh.Client({
    host: instance.PublicIpAddress,
    username: 'ubuntu',
    privateKey: sshKey.privateKey,
  })

  let connected = false
  let attempts = 5
  process.stdout.write('connecting ')
  while (!connected && attempts-- > 0) {
    try {
      await pollDelay(() => ssh.connect())
      connected = true
    } catch (error) {
      if (error.code === 'ECONNREFUSED') {
        process.stdout.write('.')
      } else {
        console.log(error)
        return
      }
    }
  }

  console.log('spawning shell')
  await ssh.shell()
  console.log('shell finished')
  await ssh.end()
}
