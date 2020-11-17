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
