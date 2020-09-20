import AWS from 'aws-sdk'
import { EventEmitter } from 'events'
import { readFileSync } from 'fs'
import { CLI } from '../ax'
import * as Ssh from './ssh'

const sshKey = {
  keyName: 'rkuhn-ec2',
  privateKey: readFileSync('/Users/rkuhn/.ssh/rkuhn-ec2.pem'),
}

// determines frequency of polling AWS APIs (e.g. waiting for instance start)
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

const netString = (x: Buffer | string) => (Buffer.isBuffer(x) ? x.toString() : x)

const createInstance = async (ec2: AWS.EC2) => {
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
  console.log('instance started')

  return instance
}

const cleanUpInstances = async (ec2: AWS.EC2) => {
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
}

export const mkInstance = async (): Promise<void> => {
  const ec2 = new AWS.EC2({ region: 'eu-central-1' })

  let instance: AWS.EC2.Instance | undefined
  if (typeof process.argv[2] === 'string') {
    const res = await ec2.describeInstances({ InstanceIds: [process.argv[2]] }).promise()
    const i = res.Reservations?.[0]?.Instances?.[0]
    instance = i !== undefined ? i : await createInstance(ec2)
  } else {
    await cleanUpInstances(ec2)
    instance = await createInstance(ec2)
  }
  if (instance === undefined) {
    return
  }

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
  process.stdout.write('\n')

  console.log('installing ActyxOS')
  await ssh.sftp((sftp) =>
    Ssh.mkProm0((cb) =>
      sftp.fastPut(
        '../dist/bin/x64/actyxos-linux',
        'actyxos',
        {
          mode: 0o755,
          step: (curr, chunk, total) => {
            process.stdout.clearLine(0)
            process.stdout.write(
              `\rprogress ${curr} / ${total} (${Math.floor((curr * 100) / total)}%)`,
            )
          },
          concurrency: 4,
        },
        cb,
      ),
    ),
  )

  console.log((await ssh.exec('pkill actyxos')).stdout)

  await pollDelay(() => Promise.resolve(1))

  const tearDown = new EventEmitter()

  const osP = new Promise<void>((res, rej) => {
    ssh.conn.exec('./actyxos', (err, channel) => {
      if (err) rej(err)
      channel.on('data', (x: Buffer | string) => {
        const s = netString(x)
        console.log('* ActyxOS: %s', s)
        if (s.indexOf('ActyxOS started') >= 0) {
          res()
        }
      })
      channel.on('close', () => {
        console.log('* ActyxOS closed')
        rej('closed')
      })
      channel.on('error', (err: Error) => {
        console.log(err)
        rej(err)
      })
      tearDown.on('end', () => {
        console.log('killing ActyxOS')
        channel.signal('TERM')
        channel.write('\x03')
      })
    })
  })

  await osP

  console.log('forwarding console port')
  const [port, server] = await ssh.forwardPort(4457)
  console.log('  console reachable on port %i', port)

  const ax = new CLI(`localhost:${port}`)
  console.log('node status', await ax.Nodes.Ls())

  tearDown.emit('end')

  await pollDelay(() => Promise.resolve(1))
  server.close()
}
