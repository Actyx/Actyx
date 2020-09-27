import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import { CLI } from '../ax'
import * as Ssh from './ssh'
import { ActyxOSNode, SshAble, Target } from './types'

const netString = (x: Buffer | string) => (Buffer.isBuffer(x) ? x.toString() : x)

// determines frequency of retrying ssh operations like connect()
const pollDelay = <T>(f: () => Promise<T>) => new Promise((res) => setTimeout(res, 2000)).then(f)

export const mkNodeLinux = async (
  name: string,
  target: Target & { kind: SshAble },
  logger: (s: string) => void = console.log,
): Promise<ActyxOSNode> => {
  // need to cast, unfortunately, to be able to get ...rest
  const { host, username, privateKey, ...rest } = <Record<string, string>>target.kind
  console.log('setting up node %s on %o', name, rest)
  const ssh = new Ssh.Client({ host, username, privateKey })

  let connected = false
  let attempts = 5
  while (!connected && attempts-- > 0) {
    try {
      await pollDelay(() => ssh.connect())
      connected = true
    } catch (error) {
      if (error.code !== 'ECONNREFUSED') {
        console.log(
          'node %s ssh connection error (remaining attempts %i): %o',
          name,
          attempts,
          error,
        )
      }
    }
  }
  if (!connected) {
    console.log('node %s ssh connection refused', name)
    throw new Error('connection refused')
  }

  console.log('node %s installing ActyxOS', name)
  await ssh.sftp(async (sftp) => {
    // ignore errors for unlink
    await Ssh.mkProm0((cb) => sftp.unlink('actyxos', () => cb(undefined)))
    await Ssh.mkProm0((cb) =>
      sftp.fastPut(
        '../dist/bin/x64/actyxos-linux',
        'actyxos',
        {
          mode: 0o755,
          step: (curr, chunk, total) => {
            const granularity = 0.1
            const now = curr / total
            const prev = (curr - chunk) / total
            const steps = Math.floor(now / granularity)
            if (prev < steps * granularity) {
              console.log('node %s ActyxOS installed %i%%', name, Math.floor(100 * now))
            }
          },
          concurrency: 4,
        },
        cb,
      ),
    )
  })

  const TIMEOUT = 10_000

  await new Promise<void>((res, rej) => {
    setTimeout(
      () => rej(new Error(`node ${name}: ActyxOS did not start within ${TIMEOUT / 1000}sec`)),
      TIMEOUT,
    )
    const lines = { stdout: '', stderr: '' }
    const log = (where: keyof typeof lines, s: string) => {
      const l = (lines[where] + s).split('\n')
      lines[where] = l.pop() || ''
      for (const line of l) {
        logger(`node ${name} ActyxOS ${where}: ${line}`)
      }
    }
    ssh.conn.exec('./actyxos', { env: { ENABLE_DEBUG_LOGS: '1' } }, (err, channel) => {
      if (err) rej(err)
      channel.on('data', (x: Buffer | string) => {
        const s = netString(x)
        log('stdout', s)
        if (s.indexOf('ActyxOS started') >= 0) {
          res()
        }
      })
      channel.stderr.on('data', (x: Buffer | string) => log('stderr', netString(x)))
      channel.on('close', () => {
        if (lines.stdout !== '') {
          log('stdout', '\n')
        }
        if (lines.stderr !== '') {
          log('stderr', '\n')
        }
        logger(`node ${name} ActyxOS channel closed`)
        rej('closed')
      })
      channel.on('error', (err: Error) => {
        logger(`node ${name} ActyxOS channel error: ${err}`)
        rej(err)
      })
    })
  })

  const [port4457, server4457] = await ssh.forwardPort(4457, (line) =>
    logger(`node ${name} ${line}`),
  )
  console.log('node %s console reachable on port %i', name, port4457)

  const [port4454, server4454] = await ssh.forwardPort(4454, (line) =>
    logger(`node ${name} ${line}`),
  )
  console.log('node %s event service reachable on port %i', name, port4454)

  const [port4243, server4243] = await ssh.forwardPort(4243, (line) =>
    logger(`node ${name} ${line}`),
  )
  console.log('node %s event service reachable on port %i', name, port4243)

  const axBinary = '../rt-master/target/release/ax'
  const axHost = `localhost:${port4457}`
  const ax = new CLI(axHost, axBinary)

  const apiConsole = `http://localhost:${port4457}/api/`
  const apiEvent = `http://localhost:${port4454}/api/`
  const opts = DefaultClientOpts()
  opts.Endpoints.ConsoleService.BaseUrl = apiConsole
  opts.Endpoints.EventService.BaseUrl = apiEvent
  const actyxOS = Client(opts)

  const apiPond = `ws://localhost:${port4243}/store_api`

  const shutdown = async () => {
    console.log('node %s shutting down', name)
    server4454.emit('end')
    server4457.emit('end')
    server4243.emit('end')
    await ssh.end()
    console.log('node %s ssh stopped', name)
    await target._private.shutdown()
    console.log('node %s instance terminated', name)
  }

  return {
    name,
    target,
    host: 'process',
    runtimes: [],
    ax,
    actyxOS,
    _private: { shutdown, axBinary, axHost, apiConsole, apiEvent, apiPond },
  }
}
