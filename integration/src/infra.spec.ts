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
  })
})
