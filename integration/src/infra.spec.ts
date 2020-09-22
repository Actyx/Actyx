import { runOnEach } from './runner/hosts'

describe('the Infrastructure', () => {
  test('must set global nodes pool', async () => {
    const status = await runOnEach([{}], false, (node) => node.ax.Nodes.Ls())
    expect(status).toHaveLength(1)
    expect(status).toMatchObject([
      {
        code: 'OK',
        result: [
          {
            appsDeployed: 0,
            appsRunning: 0,
            connection: 'reachable',
            state: 'running',
            version: '1.0.0',
          },
        ],
      },
    ])

    const settings = await runOnEach([{}], false, (node) => node.ax.Settings.Get('com.actyx.os'))
    expect(settings).toHaveLength(1)
    expect(settings).toMatchObject([
      {
        code: 'OK',
        result: {
          general: {
            logLevels: {
              apps: 'INFO',
              os: 'DEBUG',
            },
          },
          licensing: {
            apps: {},
            os: 'development',
          },
          services: {
            eventService: {
              readOnly: false,
              topic: 'Cosmos',
            },
          },
        },
      },
    ])
  })
})
