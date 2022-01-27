/**
 * @jest-environment ./dist/jest/environment
 */
import { getFreeRemotePort, occupyRemotePort } from '../../infrastructure/checkPort'
import { runOnEach, runOnEvery } from '../../infrastructure/hosts'
import { ActyxNode } from '../../infrastructure/types'
import { BoundTo, randomBinds, runActyx, runUntil, startup, withContext } from '../../util'

const startNodeAndCheckBinds = async (node: ActyxNode, params: string[]): Promise<BoundTo> => {
  const result = await startup(runActyx(node, undefined, params), node.name)
  result.process.kill()
  return result
}

const skipTarget = (node: ActyxNode): boolean =>
  // can't run multiple instances of Actyx on Android on Docker (permissions)
  node.host === 'android' || node.host === 'docker'

// FIXME almost none of these work now that Actyx runs as a service
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
      const { admin, api, swarm, log } = await startNodeAndCheckBinds(node, randomBinds)
      withContext(log, () => {
        expect(admin).toHaveLength(admin.length || 1)
        expect(api).toHaveLength(api.length || 1)
        expect(swarm).toHaveLength(swarm.length || 1)
      })
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
      const { admin, api, swarm, log } = await startNodeAndCheckBinds(node, [
        '--bind-admin',
        `0.0.0.0:${adminPort}`,
        '--bind-api',
        `0.0.0.0:${apiPort}`,
        '--bind-swarm',
        `0.0.0.0:${swarmPort}`,
      ])
      withContext(log, () => {
        expect(admin).toHaveLength(admin.length || 1)
        expect(api).toHaveLength(api.length || 1)
        expect(swarm).toHaveLength(swarm.length || 1)
        expect(admin.every(([_, port]) => port === adminPort)).toBeTruthy()
        expect(api.every(([_, port]) => port === apiPort)).toBeTruthy()
        expect(swarm.every(([_, port]) => port === swarmPort)).toBeTruthy()
      })
    }))

  it('indicate a successful start', () =>
    runOnEvery(async (node) => {
      if (skipTarget(node)) {
        return
      }
      const proc = await runUntil(
        runActyx(node, undefined, randomBinds),
        node.name,
        ['NODE_STARTED_BY_HOST'],
        10_000,
      )
      expect(Array.isArray(proc)).toBeTruthy()
      expect(proc).toContainEqual(expect.stringContaining('NODE_STARTED_BY_HOST'))
    }))

  it('indicate shutdown', () =>
    runOnEvery(async (node) => {
      if (node.target.kind.type !== 'local' || skipTarget(node)) {
        // It's not straight-forward to forward the signal via SSH
        return
      }
      const { process } = await runActyx(node, undefined, randomBinds)
      const logs: string[] = await new Promise((res, rej) => {
        const buffer: string[] = []
        process.stderr?.on('data', (buf) => buffer.push(buf.toString('utf8')))
        process.stderr?.on('error', (err) => rej(err))
        process.stderr?.on('end', () => res(buffer))
        setTimeout(() => process.kill('SIGTERM'), 500)
      })
      // eslint-disable-next-line no-control-regex
      expect(logs.join('').replace(/\u001b\[[^a-z]*[a-z]/g, '')).toEqual(
        expect.stringContaining(
          'NODE_STOPPED_BY_HOST: Actyx is stopped. The shutdown was either initiated automatically by the host or intentionally by the user.',
        ),
      )
      expect(process.killed).toBeTruthy()
    }))

  const services = ['Admin', 'API', 'Swarm']
  services.map((x) =>
    it(`should error on occupied ports (${x})`, (done) => {
      if (x === 'Admin') {
        done()
        return
      }

      runOnEach([{ host: 'process', os: 'linux' }], async (node) => {
        // Tracking issue for Windows: https://github.com/Actyx/Cosmos/issues/5850
        const port = await getFreeRemotePort(node.target)
        const server = occupyRemotePort(node.target, port)
        let hot = true
        server.catch((e) => hot && done(e))

        const notX = services
          .filter((y) => y !== x)
          .flatMap((y) => [`--bind-${y.toLowerCase()}`, '0.0.0.0:0'])
        const proc = await runUntil(
          runActyx(node, undefined, [`--bind-${x.toLowerCase()}`, `0.0.0.0:${port}`].concat(notX)),
          node.name,
          [],
          10_000,
        )
        hot = false
        server.kill('SIGTERM')

        if (Array.isArray(proc)) {
          throw new Error(`timed out, port=${port}:\n${proc.join('\n')}`)
        }
        const logs = proc.stderr

        // eslint-disable-next-line no-control-regex
        expect(logs.replace(/\u001b\[[^a-z]*[a-z]/g, '')).toMatch(
          'NODE_STOPPED_BY_NODE: ERR_PORT_COLLISION',
        )
        expect(logs).toMatch(
          `Actyx shut down because it could not bind to port /ip4/0.0.0.0/tcp/${port.toString()}. Please specify a different ${x} port.`,
        )
      }).then(() => done(), done)
    }),
  )

  // FIXME make shutdown reliable and merge back into the above test. https://github.com/Actyx/Cosmos/issues/7106
  it(`should error on occupied ports (Admin FIXME)`, (done) => {
    const x = 'Admin'
    runOnEach([{ host: 'process', os: 'linux' }], async (node) => {
      const port = await getFreeRemotePort(node.target)
      const server = occupyRemotePort(node.target, port)
      let hot = true
      server.catch((e) => hot && done(e))

      const notX = services
        .filter((y) => y !== x)
        .flatMap((y) => [`--bind-${y.toLowerCase()}`, '0.0.0.0:0'])
      const proc = await runUntil(
        runActyx(node, undefined, [`--bind-${x.toLowerCase()}`, `0.0.0.0:${port}`].concat(notX)),
        node.name,
        ['Please specify a different'],
        10_000,
      )
      hot = false
      server.kill('SIGTERM')

      if (!Array.isArray(proc)) {
        throw new Error('Expected array of logs')
      }
      const logs = proc.join('\n')

      expect(logs).toMatch('NODE_STOPPED_BY_NODE: ERR_PORT_COLLISION')
      expect(logs).toMatch(
        `Actyx shut down because it could not bind to port /ip4/0.0.0.0/tcp/${port.toString()}. Please specify a different Admin port.`,
      )
    }).then(() => done(), done)
  })

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
      const { admin, api, swarm, log } = await startNodeAndCheckBinds(n, [
        '--bind-admin',
        `127.0.0.1:${adminPort.toString()}`,
        '--bind-api',
        `127.0.0.1:${apiPort.toString()}`,
        '--bind-swarm',
        `127.0.0.1:${swarmPort.toString()}`,
      ])
      withContext(log, () => {
        expect(admin).toStrictEqual([['127.0.0.1', adminPort]])
        expect(api).toStrictEqual([['127.0.0.1', apiPort]])
        expect(swarm).toStrictEqual([['127.0.0.1', swarmPort]])
      })
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
      const { admin, api, swarm, log } = await startNodeAndCheckBinds(n, [
        '--bind-admin',
        `/ip4/127.0.0.1/tcp/${adminPort.toString()}`,
        '--bind-api',
        `/ip4/127.0.0.1/tcp/${apiPort.toString()}`,
        '--bind-swarm',
        `/ip4/127.0.0.1/tcp/${swarmPort.toString()}`,
      ])
      withContext(log, () => {
        expect(admin).toStrictEqual([['127.0.0.1', adminPort]])
        expect(api).toStrictEqual([['127.0.0.1', apiPort]])
        expect(swarm).toStrictEqual([['127.0.0.1', swarmPort]])
      })
    }))

  it('should refuse to run in an already used workdir', () =>
    runOnEvery(async (node) => {
      if (skipTarget(node)) {
        return
      }

      const out = await runUntil(
        runActyx(node, node._private.workingDir, []),
        node.name,
        [],
        10_000,
      )
      if (Array.isArray(out)) {
        throw new Error(`timed out:\n${out.join('\n')}`)
      }
      expect(out.stderr).toMatch(
        `data directory \`${node._private.workingDir}\` is locked by another Actyx process`,
      )
    }))
})
