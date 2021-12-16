import { Pond } from '@actyx/pond'
import { execaCommand } from 'execa'
import { MyGlobal } from '../../../jest/setup'
import { assertOK } from '../../assertOK'
import { runOnAll, runOnEach } from '../../infrastructure/hosts'

describe('the Infrastructure', () => {
  test('must create global nodes pool', async () => {
    const status = await runOnEach([{}], async (node) => {
      const response = assertOK(await node.ax.nodes.ls())
      const axNodeSetup = (<MyGlobal>global).axNodeSetup
      let gitHash = axNodeSetup.gitHash
      const dirty = await execaCommand('git status --porcelain').then((x) => x.stdout)
      if (dirty !== '') {
        gitHash = `${gitHash}_dirty`
      }

      expect(response).toMatchObject({
        code: 'OK',
        result: [
          {
            connection: 'reachable',
            version: {
              profile: 'release',
              target: `${node.target.os}-${node.target.arch}`,
              gitHash,
            },
          },
        ],
      })
    })

    expect(status).toHaveLength(1)
  })

  test('must set up global nodes', async () => {
    const settings = await runOnEach([{}], (node) => node.ax.settings.get('/'))
    expect(settings).toMatchObject([
      {
        code: 'OK',
        result: {
          admin: {
            logLevels: {
              node: 'DEBUG',
            },
          },
          licensing: {
            apps: {},
            node: 'development',
          },
          api: {
            events: {
              readOnly: false,
            },
          },
          swarm: {
            topic: 'Cosmos integration',
          },
        },
      },
    ])
    expect(settings).toHaveLength(1)
  })

  test('must test Pond v2', async () => {
    const result = await runOnAll([{}], async ([node]) => {
      const pond = await Pond.of(
        {
          appId: 'com.example.infra-test',
          displayName: 'Our Infra Test',
          version: '1.0.0',
        },
        { actyxPort: node._private.apiPort },
        {},
      )
      return pond.info().nodeId
    })
    expect(typeof result).toBe('string')
  })
})

describe('scripts', () => {
  test('must allow running sh scripts on linux', async () => {
    await runOnEach([{ os: 'linux' }], async (node) => {
      const script = String.raw`if [[ $(expr 1 + 1) -eq 2 ]]
then
  echo "yay"
  exit 0
else
  exit 1
fi`
      const result = await node.target.execute(script, []).process
      expect(result.exitCode).toBe(0)
      expect(result.stdout).toBe('yay')
      expect(result.stderr).toBe('')
    })
  })
  test('must allow running powershell scripts on windows', async () => {
    await runOnEach([{ os: 'windows' }], async (node) => {
      const script = String.raw`$val = 0
while ($val -lt 10) {
  $val++
}
$val + 32
exit 0`
      const result = await node.target.execute(script, []).process
      expect(result.exitCode).toBe(0)
      expect(result.stdout).toBe('42')
      expect(result.stderr).toBe('')
    })
  })
})
