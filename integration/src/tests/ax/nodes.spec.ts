import { assertOK } from '../../assertOK'
import { CLI } from '../../cli'
import { runOnEvery } from '../../infrastructure/hosts'
import { currentAxBinary } from '../../infrastructure/settings'
import { mkAxWithUnreachableNode } from '../../stubs'
import { MyGlobal } from '../../../jest/setup'
import execa from 'execa'
import { runActyx } from '../../util'

describe('ax nodes', () => {
  describe('ls', () => {
    test('return Ok and result with connection hostUnreachable', async () => {
      const ax = await mkAxWithUnreachableNode()
      const response = await ax.nodes.ls()
      expect(response).toMatchObject({
        code: 'OK',
        result: [
          {
            connection: 'unreachable',
            host: expect.any(String),
          },
        ],
      })
    })

    test('return OK and result with connection reachable', async () => {
      await runOnEvery(async (node) => {
        const response = assertOK(await node.ax.nodes.ls())
        const axNodeSetup = (<MyGlobal>global).axNodeSetup
        let gitHash = axNodeSetup.gitHash
        const dirty = await execa.command('git status --porcelain').then((x) => x.stdout)
        if (dirty !== '') {
          gitHash = `${gitHash}_dirty`
        }

        let version
        if (node.host === 'android' || node.host === 'docker') {
          version = expect.any(String)
        } else {
          const out = await (await runActyx(node, undefined, ['--version']))[0]
          version = out.stdout.replace('actyx ', '').split('-')[0]
        }

        // Android is running inside an emulator, currently hardcoded to x86
        const target =
          node.host === 'android'
            ? expect.stringContaining('android')
            : `${node.target.os}-${node.target.arch}`

        const responseShape = [
          {
            connection: 'reachable',
            host: node._private.axHost,
            nodeId: expect.any(String),
            displayName: node.name,
            startedIso: expect.any(String),
            startedUnix: expect.any(Number),
            version: {
              profile: 'release',
              target,
              version,
              gitHash,
            },
          },
        ]
        expect(response.result).toMatchObject(responseShape)
      })
    })

    test('return OK and result with unauthorized', async () => {
      await runOnEvery(async (node) => {
        // This will generate a CLI with a different than private key the node
        // was setup with
        const unauthorizedCli = await CLI.build(node._private.axHost, await currentAxBinary())
        const response = assertOK(await unauthorizedCli.nodes.ls())
        const responseShape = [
          {
            connection: 'unauthorized',
            host: expect.any(String),
          },
        ]
        expect(response.result).toMatchObject(responseShape)
      })
    })
  })

  describe('inspect', () => {
    test('show consistent outputs', async () => {
      await runOnEvery(async (node) => {
        const addrs = assertOK(await node.ax.nodes.inspect()).result.adminAddrs

        expect(addrs.length).toBeGreaterThan(1)
        expect(addrs).toContainEqual(expect.stringContaining('/127.0.0.1/'))

        const regex = new RegExp('/(ip[46])(?:/[^/]+){2}/(\\d+)')
        const ports = addrs.map((a) => a.match(regex)?.slice(1, 3) || ['', ''])

        const protos = { ip4: [] as number[], ip6: [] as number[] }
        for (const [proto, port] of ports) {
          const p = Number(port)
          expect(p).toBeGreaterThan(0)
          switch (proto) {
            case 'ip4':
            case 'ip6':
              protos[proto].push(p)
              break
            default:
              throw new Error(`unknown proto in admin addr: ${proto}\nresponse was ${addrs}`)
          }
        }

        for (const p of protos.ip4) {
          expect(p).toBe(protos.ip4[0])
        }
        for (const p of protos.ip6) {
          expect(p).toBe(protos.ip6[0])
        }
      })
    })
  })
})
