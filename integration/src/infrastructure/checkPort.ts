import net from 'net'

export const portInUse = (port: number): Promise<boolean> =>
  new Promise((res) => {
    const server = net.createServer()
    server.once('error', () => res(true))
    server.once('listening', () => {
      server.close()
      res(false)
    })
    server.listen(port)
  })

export const getFreePort = (): Promise<number> =>
  new Promise((res, rej) => {
    const server = net.createServer()
    server.once('error', rej)
    server.once('listening', () => {
      const addr = server.address()
      if (typeof addr !== 'object' || addr === null) {
        server.close()
        rej(new Error(`listening server address was ${addr}`))
      } else {
        server.close(() => res(addr.port))
      }
    })
    server.listen()
  })
