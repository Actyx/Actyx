import { CLI } from '../ax'
import * as Ssh from './ssh'
import { ActyxOSNode, SshAble, Target } from './types'

const netString = (x: Buffer | string) => (Buffer.isBuffer(x) ? x.toString() : x)

// determines frequency of retrying ssh operations like connect()
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

export const mkNodeLinux = async (
  name: string,
  target: Target & { kind: SshAble },
): Promise<ActyxOSNode> => {
  const { host, username, privateKey } = target.kind
  const ssh = new Ssh.Client({ host, username, privateKey })

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
        throw new Error(`connection error: ${error}`)
      }
    }
  }
  if (!connected) {
    throw new Error('connection refused')
  }
  process.stdout.write('\n')

  console.log('installing ActyxOS')
  await ssh.sftp(async (sftp) => {
    await Ssh.mkProm0((cb) => sftp.unlink('actyxos', cb))
    return Ssh.mkProm0((cb) =>
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
    )
  })
  process.stdout.write('\n')

  await new Promise<void>((res, rej) => {
    ssh.conn.exec('./actyxos', { env: { ENABLE_DEBUG_LOGS: '1' } }, (err, channel) => {
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
    })
  })

  console.log('forwarding console port')
  const [port, server] = await ssh.forwardPort(4457)
  console.log('  console reachable on port %i', port)

  const ax = new CLI(`localhost:${port}`)

  const shutdown = () => {
    server.emit('end')
    ssh.end()
    if ('shutdown' in target) {
      target.shutdown()
    }
  }

  return { name, target, host: 'process', runtimes: [], ax, shutdown }
}
