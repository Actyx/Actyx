/* eslint-disable @typescript-eslint/no-empty-function */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { createServer, Server } from 'net'
import * as SshClient from 'ssh2'
import { Duplex } from 'stream'
import * as tty from 'tty'

export enum ExecOutput {
  Collect,
  Stream,
  Tee,
}

export interface ExecOptions {
  output?: ExecOutput
}

export interface ExecResult {
  signal?: string
  code?: number
  stdout: string
  stderr: string
}

export const mkProm = <T>(f: (cb: (err: any, value: T) => void) => void): Promise<T> =>
  new Promise<T>((res, rej) =>
    f((err: any, value: T) => {
      if (err !== undefined) {
        rej(err)
      } else {
        res(value)
      }
    }),
  )

export const mkProm0 = (f: (cb: (err: any) => void) => void): Promise<void> =>
  new Promise<void>((res, rej) =>
    f((err: any) => {
      if (err) {
        rej(err)
      } else {
        res()
      }
    }),
  )

/**
 * Low level async wrapper. Service builds on this for a more terse API.
 */
export class Client {
  config: SshClient.ConnectConfig
  conn: SshClient.Client
  private _endPromise!: Promise<boolean>

  constructor(config: SshClient.ConnectConfig) {
    this.config = config
    this.conn = new SshClient.Client()
  }

  connect(): Promise<void> {
    const prom = new Promise<void>((res, rej) => {
      this.conn.on('ready', () => res())
      this.conn.on('error', rej)
      this.conn.on('end', () => {
        rej(new Error('Connection ended'))
      })
    })
    this._endPromise = new Promise<boolean>((res) => {
      this.conn.on('error', () => {
        res(true)
      })
      this.conn.on('end', () => {
        res(true)
      })
    })
    this.conn.connect(this.config)
    return prom
  }

  forwardOut(
    srcIP: string,
    srcPort: number,
    dstIP: string,
    dstPort: number,
  ): Promise<SshClient.Channel> {
    const prom = new Promise<SshClient.Channel>((res, rej) => {
      this.conn.forwardOut(srcIP, srcPort, dstIP, dstPort, (err, chan) => {
        if (err) rej(err)
        res(chan)
      })
    })
    return prom
  }

  private id = 1

  forwardPort(
    dstPort: number,
    logger: (s: string) => void = console.log,
  ): Promise<[number, Server]> {
    const prom = new Promise<[number, Server]>((res, rej) => {
      const connections: Duplex[] = []
      const server = createServer((conn) => {
        const myId = this.id++
        logger(
          `new connection ${myId} from ${conn.remoteAddress}:${conn.remotePort} -> ${conn.localAddress}:${conn.localPort}, forwarding to port ${dstPort}`,
        )
        conn.setNoDelay()
        const port = conn.localPort
        this.conn.forwardOut('127.0.0.1', port, '127.0.0.1', dstPort, (err, channel) => {
          if (err) {
            conn.destroy(err)
            return
          }
          conn.pipe(channel).pipe(conn)
          logger(`pipe established ${myId}`)
          connections.push(conn, channel)
          conn.on('error', (err) => logger(`incoming connection ${myId} error: ${err.message}`))
          conn.on('end', () => {
            logger(`incoming connection ${myId} closed`)
            channel.end()
          })
          channel.on('error', (err: any) =>
            logger(`forwarded connection ${myId} error: ${err.message}`),
          )
          channel.on('end', () => {
            logger(`forwarded connection ${myId} closed`)
            conn.end()
          })
        })
      })
      server.on('end', () => {
        logger(`forwarder for port ${dstPort} closing down`)
        server.close()
        connections.forEach((conn) => conn.end())
        connections.splice(0, connections.length)
      })
      server.on('error', rej)
      server.listen(0, '127.0.0.1', () => {
        const addr = server.address()
        if (addr === null || typeof addr === 'string') {
          rej(new Error(`got weird address ${addr} after listen event`))
        } else {
          res([addr.port, server])
        }
      })
    })
    return prom
  }

  async pty(cmd: string): Promise<void> {
    const stream = await this.stream(cmd, true)
    this.pipeStream(stream)
    await new Promise((res) => {
      stream.on('close', () => {
        this.unpipeStream(stream)
        res()
      })
    })
  }

  stream(cmd: string, pty?: boolean): Promise<SshClient.Channel> {
    const prom = new Promise<SshClient.Channel>((res, rej) => {
      this.conn.exec(cmd, { pty: !!pty || undefined }, (err, stream) => {
        if (err) {
          rej(err)
          return
        }
        res(stream)
      })
    })
    return prom
  }

  shell(): Promise<boolean> {
    const prom = new Promise<boolean>((res, rej) => {
      this.conn.shell({ term: process.env.TERM || 'vt100' }, (err, stream) => {
        if (err) {
          rej(err)
          return
        }
        this.pipeStream(stream)
        stream.on('close', () => {
          this.unpipeStream(stream)
          res(true)
        })
      })
    })
    return prom
  }

  exec(cmd: string, options?: ExecOptions): Promise<ExecResult> {
    const ensureOpts = options || { output: ExecOutput.Collect }
    const result: ExecResult = {
      stderr: '',
      stdout: '',
    }
    const prom: Promise<ExecResult> = new Promise((res, rej) => {
      this.conn.exec(cmd, (err, stream) => {
        if (err) {
          rej(err)
          return
        }
        stream.on('close', (code: number, signal: string) => {
          result.code = code
          result.signal = signal
          stream.end()
        })
        stream.on('end', () => {
          res(result)
        })
        stream.on('data', (data: string) => {
          this.handleExecData('stdout', result, data, ensureOpts)
        })
        stream.stderr.on('data', (data) => {
          this.handleExecData('stderr', result, data, ensureOpts)
        })
      })
    })
    return prom
  }

  sftp<T>(f: (x: SshClient.SFTPWrapper) => Promise<T> | T): Promise<T> {
    const prom = new Promise<T>((res, rej) => {
      this.conn.sftp((err, sftp) => {
        if (err) {
          rej(err)
          return
        }
        try {
          Promise.resolve(f(sftp)).then(res).catch(rej)
        } catch (err) {
          rej(err)
        }
      })
    })
    return prom
  }

  private handleExecData(
    stream: 'stdout' | 'stderr',
    result: ExecResult,
    data: string | Buffer,
    options: ExecOptions,
  ) {
    switch (options.output) {
      case ExecOutput.Collect:
        result[stream] += data
        break
      case ExecOutput.Stream:
        process[stream].write(data)
        break
      case ExecOutput.Tee:
        result[stream] += data
        process[stream].write(data)
        break
    }
  }

  private pipeStream(stream: SshClient.Channel) {
    const stdout = process.stdout as tty.WriteStream
    const stdin = process.stdin as tty.ReadStream
    const streamStderr = (stream.stderr as any) as tty.ReadStream
    stdin.setRawMode(true)
    stream.pipe(stdout)
    streamStderr.pipe(process.stderr)
    stdin.pipe(stream)
    stream.once('data', () => {
      ;(<any>stream).setWindow(stdout.rows, stdout.columns, null, null)
    })
    process.stdout.on('resize', () => {
      ;(<any>stream).setWindow(stdout.rows, stdout.columns, null, null)
    })
  }

  private unpipeStream(stream: SshClient.Channel) {
    const stdin = process.stdin as tty.ReadStream
    const streamStdErr = (stream.stderr as any) as tty.ReadStream
    stdin.unpipe()
    stream.unpipe()
    streamStdErr.unpipe()
    stdin.setRawMode(false)
    stdin.unref()
  }

  async end(): Promise<void> {
    this.conn.end()
    await this._endPromise
    this.conn.removeAllListeners()
  }
}
