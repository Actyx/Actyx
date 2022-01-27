import util from 'util'
import net from 'net'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const mkLog =
  (node: string) =>
  (msg: string, ...rest: any[]): void => {
    process.stdout.write(util.format(`node ${node} ${msg}\n`, ...rest))
  }

export const randIdentifier = (): string => Math.random().toString(36).substring(7)

export const portBound = (port: number): Promise<void> =>
  new Promise((res, rej) => {
    let tries = 20
    const check = () => {
      const sock = new net.Socket()
      sock
        .connect({ port })
        .on('connect', () => {
          sock.destroy()
          res()
        })
        .on('error', () => {
          sock.destroy()
          tries -= 1
          if (tries === 0) {
            rej(new Error('timed out waiting for port forwarding'))
          } else {
            setTimeout(check, 500)
          }
        })
    }
    setTimeout(check, 0)
  })
