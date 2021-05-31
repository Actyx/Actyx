import { Observable } from 'rxjs'
import { startEphemeralNode, startEphemeralProcess } from '../../infrastructure'
import { getFreeRemotePort, occupyRemotePort } from '../../infrastructure/checkPort'
import { runOnEvery } from '../../infrastructure/hosts'
import { ActyxNode } from '../../infrastructure/types'

const adminExtract = (s: string): [string, number] | undefined => {
  const match = s.match(
    new RegExp(
      '(?:ADMIN_API_BOUND: Admin API bound to /ip4/)([0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3})(?:/tcp/)(\\d+)',
    ),
  )
  return match ? [match[1], Number(match[2])] : undefined
}
const apiExtract = (s: string): [string, number] | undefined => {
  const match = s.match(
    new RegExp(
      '(?:API_BOUND: API bound to )([0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3})\\:(\\d*)',
    ),
  )
  return match ? [match[1], Number(match[2])] : undefined
}
const swarmExtract = (s: string): [string, number] | undefined => {
  const match = s.match(
    new RegExp(
      '(?:SWARM_SERVICES_BOUND: Swarm Services bound to /ip4/)([0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3})(?:/tcp/)(\\d+)',
    ),
  )
  return match ? [match[1], Number(match[2])] : undefined
}

type BoundTo = {
  admin: [string, number][]
  api: [string, number][]
  swarm: [string, number][]
}

const randomBinds = ['--bind-admin', '0', '--bind-api', '0', '--bind-swarm', '0']
const startNode = (node: ActyxNode, params: string[] = randomBinds) =>
  startEphemeralNode(node.target, node._private.actyxBinaryPath, params)
const startNodeAndCheckBinds = async (node: ActyxNode, params: string[]): Promise<BoundTo> => {
  const testNode = await startEphemeralNode(node.target, node._private.actyxBinaryPath, params)
  const result: BoundTo = { admin: [], api: [], swarm: [] }
  await testNode
    // .do(console.log)
    .filter((x) => x.includes('bound to'))
    .takeUntil(Observable.timer(5000).first())
    .forEach((x) => {
      const admin = adminExtract(x)
      if (admin) {
        result.admin.push(admin)
      }
      const api = apiExtract(x)
      if (api) {
        result.api.push(api)
      }
      const swarm = swarmExtract(x)
      if (swarm) {
        result.swarm.push(swarm)
      }
    })

  return result
}
const skipTarget = (node: ActyxNode): boolean =>
  // can't run multiple instances of Actyx on Android on Docker (permissions)
  node.host === 'android' || node.host === 'docker'

describe('node lifecycle', () => {
  // These ports are potentially quite contended on CI servers.
  // This is implicitly tested anyway by running Actyx on Linux and Windows.
  it.skip('should bind to default ports interfaces', () =>
    runOnEvery(async (n) => {
      const { admin, api, swarm } = await startNodeAndCheckBinds(n, [])
      ;[admin, api, swarm].forEach((x) => expect(x.length > 0).toBeTruthy())
      expect(admin.every(([_, port]) => port === 4458)).toBeTruthy()
      expect(admin.some(([ip, _]) => ip !== '127.0.0.1')).toBeTruthy()
      expect(api.every(([ip, port]) => port === 4454 && ip === '127.0.0.1')).toBeTruthy()
      expect(swarm.every(([ip, port]) => port === 4001 && ip !== '127.0.0.1')).toBeTruthy()
    }))

  it('should bind to os provided ports', () =>
    runOnEvery(async (node) => {
      if (skipTarget(node)) {
        return
      }
      const { admin, api, swarm } = await startNodeAndCheckBinds(node, randomBinds)
      ;[admin, api, swarm].forEach((x) => expect(x.length > 0).toBeTruthy())
    }))

  it('should bind to specified ports', () =>
    runOnEvery(async (node) => {
      if (skipTarget(node)) {
        return
      }
      const [adminPort, apiPort, swarmPort] = await Promise.all([
        getFreeRemotePort(node.target),
        getFreeRemotePort(node.target),
        getFreeRemotePort(node.target),
      ])
      const { admin, api, swarm } = await startNodeAndCheckBinds(node, [
        '--bind-admin',
        adminPort.toString(),
        '--bind-api',
        apiPort.toString(),
        '--bind-swarm',
        swarmPort.toString(),
      ])
      ;[admin, api, swarm].forEach((x) => expect(x.length > 0).toBeTruthy())
      expect(admin.every(([_, port]) => port === adminPort)).toBeTruthy()
      expect(api.every(([_, port]) => port === apiPort)).toBeTruthy()
      expect(swarm.every(([_, port]) => port === swarmPort)).toBeTruthy()
    }))

  it('indicate a successful start', () =>
    runOnEvery(async (n) => {
      if (skipTarget(n)) {
        return
      }
      const node = await startNode(n)
      await node
        .filter((x) => x.includes('NODE_STARTED_BY_HOST'))
        .first()
        .toPromise()
    }))

  it('indicate shutdown', () =>
    runOnEvery(async (n) => {
      if (n.target.kind.type !== 'local') {
        // It's not straight-forward to forward the signal via SSH
        return
      }
      const node = (await startEphemeralProcess(n.target, n._private.actyxBinaryPath, []))[0]
      const logs: string[] = await new Promise((res, rej) => {
        const buffer: string[] = []
        node.stdout?.on('data', (buf) => buffer.push(buf.toString('utf8')))
        node.stdout?.on('error', (err) => rej(err))
        node.stdout?.on('end', () => res(buffer))
        setTimeout(() => node.kill('SIGTERM'), 500)
      })
      expect(
        logs.find((x) =>
          x.includes(
            'NODE_STOPPED_BY_HOST: Actyx is stopped. The shutdown was either initiated automatically by the host or intentionally by the user.',
          ),
        ),
      ).not.toBeUndefined()
      expect(node.killed).toBeTruthy()
    }))

  it('should error on occupied ports', () =>
    runOnEvery(async (n) => {
      if (skipTarget(n) || n.target.os === 'windows') {
        // Tracking issue for Windows: https://github.com/Actyx/Cosmos/issues/5850
        return
      }
      const services = ['Admin', 'API', 'Swarm']
      await Promise.all(
        services.map(async (x) => {
          const port = await getFreeRemotePort(n.target)
          const server = occupyRemotePort(n.target, port)
          await new Promise((res) => setTimeout(res, 500))
          const notX = services
            .filter((y) => y !== x)
            .flatMap((y) => [`--bind-${y.toLowerCase()}`, '0'])
          const node = await startNode(
            n,
            [`--bind-${x.toLowerCase()}`, port.toString()].concat(notX),
          )
          const logs = await node.toArray().toPromise()
          server.kill('SIGTERM')
          expect(
            logs.find((y) => y.includes('NODE_STOPPED_BY_NODE: ERR_PORT_COLLISION')),
          ).toBeTruthy()
          expect(
            logs.find((y) =>
              y.includes(
                `Actyx shut down because it could not bind to port ${port.toString()}. Please specify a different ${x} port.`,
              ),
            ),
          ).toBeTruthy()
        }),
      )
    }))

  it('should work with host:port combinations', () =>
    runOnEvery(async (n) => {
      if (skipTarget(n)) {
        return
      }
      const [adminPort, apiPort, swarmPort] = await Promise.all([
        getFreeRemotePort(n.target),
        getFreeRemotePort(n.target),
        getFreeRemotePort(n.target),
      ])
      const { admin, api, swarm } = await startNodeAndCheckBinds(n, [
        '--bind-admin',
        `127.0.0.1:${adminPort.toString()}`,
        '--bind-api',
        `127.0.0.1:${apiPort.toString()}`,
        '--bind-swarm',
        `127.0.0.1:${swarmPort.toString()}`,
      ])
      expect(admin).toStrictEqual([['127.0.0.1', adminPort]])
      expect(api).toStrictEqual([['127.0.0.1', apiPort]])
      expect(swarm).toStrictEqual([['127.0.0.1', swarmPort]])
    }))

  it('should work with multiaddrs', () =>
    runOnEvery(async (n) => {
      if (skipTarget(n)) {
        return
      }
      const [adminPort, apiPort, swarmPort] = await Promise.all([
        getFreeRemotePort(n.target),
        getFreeRemotePort(n.target),
        getFreeRemotePort(n.target),
      ])
      const { admin, api, swarm } = await startNodeAndCheckBinds(n, [
        '--bind-admin',
        `/ip4/127.0.0.1/tcp/${adminPort.toString()}`,
        '--bind-api',
        `/ip4/127.0.0.1/tcp/${apiPort.toString()}`,
        '--bind-swarm',
        `/ip4/127.0.0.1/tcp/${swarmPort.toString()}`,
      ])
      expect(admin).toStrictEqual([['127.0.0.1', adminPort]])
      expect(api).toStrictEqual([['127.0.0.1', apiPort]])
      expect(swarm).toStrictEqual([['127.0.0.1', swarmPort]])
    }))
})
