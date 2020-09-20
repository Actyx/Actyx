import * as SshClient from 'ssh2'
import * as tty from 'tty'
import Queue from 'promise-queue'

export enum ExecOutput {
  Collect,
  Stream,
  Tee,
}

export interface IExecOptions {
  output?: ExecOutput
}

export interface IExecResult {
  signal?: string
  code?: number
  stdout: string
  stderr: string
}

export interface ISsh {
  pty(cmd: string): Promise<void>
  stream(cmd: string, pty?: boolean): Promise<SshClient.Channel>
  shell(): Promise<boolean>
  exec(cmd: string, options?: IExecOptions): Promise<IExecResult>
}

export class Util {
  /**
   * Takes an array of Client and tunnels through them using netcat from first to last
   */
  static async Tunnel(clients: Client[]) {
    for (let x = 0; x < clients.length - 1; x++) {
      const c = clients[x]
      const cNext = clients[x + 1]
      await c.connect()
      /** Netcat no longer necessary but we could add swappable tunnel sock providers in the future */
      // let nc = `nc -w 10 -v ${cNext.config.host} ${cNext.config.port || "22"}`
      // let sock = await c.stream(nc)
      // await Util.netCat(sock)
      const sock = await c.forwardOut('192.168.1.1', 8000, cNext.config.host!, cNext.config.port!)
      cNext.config.sock = sock
      cNext.config.host = undefined
    }
    const cFinal = clients[clients.length - 1]
    await cFinal.connect()
  }

  private static netCat(stream: SshClient.Channel): Promise<boolean> {
    let stderr = ''
    const prom = new Promise<boolean>((res, rej) => {
      stream.stderr.on('data', (data) => {
        stderr += data
        if (/(succeeded|open)/.test(data)) {
          res(true)
        }
      })
      stream.on('data', (data) => {
        console.log('netcat', data)
      })
      stream.on('end', () => {
        rej(new Error(`Netcat connection error:\n${stderr}`))
      })
    })
    return prom
  }
}

/**
 * Service build on top of Client and Util and provides a sugary API.
 */
export class Service implements ISsh {
  configDefaults: SshClient.ConnectConfig = {}
  private _clients: Client[] = []
  private _configs: SshClient.ConnectConfig[] = []
  private get _client(): Client {
    return this._clients[this._clients.length - 1]
  }

  private _execQueue = new Queue(10, Infinity)

  /** Connect to final destination */
  async connect() {
    this._configs.forEach((c) => {
      this._clients.push(new Client({ ...c }))
    })
    await Util.Tunnel(this._clients)
  }

  /** Add a connection to a the tunnel chain */
  tunnel(config: SshClient.ConnectConfig) {
    this.addConnection(config)
    return this
  }

  host(config: SshClient.ConnectConfig) {
    this.addConnection(config)
    return this
  }

  public addConnection(config: SshClient.ConnectConfig) {
    const defaultedConfig = { ...this.configDefaults, ...config }
    this._configs.push(defaultedConfig)
  }

  /** Set connection defaults for use an all subsequent calls */
  defaults(config: SshClient.ConnectConfig) {
    this.configDefaults = config
    return this
  }

  async tri(cmd: string, options?: IExecOptions): Promise<boolean> {
    const res = await this.exec(cmd, options)
    if (res.code === 0) {
      return true
    } else {
      return false
    }
  }

  /**
   * Exec multiple commands concurrently utilizing ssh channels.
   * Currently 10 commands will be executed concurrently, which is
   * the default number of channels for ssh on many distros.
   */
  async exec(cmd: string, options?: IExecOptions): Promise<IExecResult> {
    const retries = 0,
      maxRetry = 5
    while (retries < maxRetry) {
      try {
        return await this._execQueue.add(() => {
          return this._client.exec(cmd, options)
        })
      } catch (e) {
        // Ocassionally a channel is freed on the client end however the remote server has not yet cleaned it up.
        if (/Channel open failure/.test(e.message)) {
          console.error(`Channel Open failure: ${retries}`)
        } else {
          throw e
        }
      }
    }
    throw new Error('SSH exec exceeded retries.')
  }

  /** Execute command with a sudo terminal. */
  async pty(cmd: string) {
    return this._client.pty(cmd)
  }

  /**
   * Run a command and return the ssh stream. Usefull for commands with lots of output
   * and for interacting with output as it is produced.
   */
  stream(cmd: string, pty?: boolean): Promise<SshClient.Channel> {
    return this._client.stream(cmd, pty)
  }

  /** Opens the users shell and wires up stdin/stdout. */
  shell() {
    return this._client.shell()
  }

  /** Closes open client connections. This attempts to allow the service settings to be reused. */
  async end() {
    for (const c of this._clients) {
      await c.end()
    }
    this._clients = []
  }
}

/**
 * Low level async wrapper. Service builds on this for a more terse API.
 */
export class Client implements ISsh {
  config: SshClient.ConnectConfig
  conn: SshClient.Client
  private _execQueue: any
  private _endPromise!: Promise<boolean>

  constructor(config: SshClient.ConnectConfig) {
    this.config = config
    this.conn = new SshClient.Client()
  }

  connect() {
    const prom = new Promise((res, rej) => {
      this.conn.on('ready', () => res())
      this.conn.on('error', rej)
      this.conn.on('end', () => {
        rej(new Error('Connection ended'))
      })
    })
    this._endPromise = new Promise<boolean>((res, rej) => {
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

  async pty(cmd: string) {
    const stream = await this.stream(cmd, true)
    this.pipeStream(stream)
    await new Promise((res, rej) => {
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

  shell() {
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

  exec(cmd: string, options?: IExecOptions): Promise<IExecResult> {
    const ensureOpts = options || { output: ExecOutput.Collect }
    const result: IExecResult = {
      stderr: '',
      stdout: '',
    }
    const prom: Promise<IExecResult> = new Promise((res, rej) => {
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

  private handleExecData(
    stream: 'stdout' | 'stderr',
    result: IExecResult,
    data: string | Buffer,
    options: IExecOptions,
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
