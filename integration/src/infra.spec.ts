import { Event, EventDraft } from '@actyx/os-sdk'
import { allNodeNames, runOnEach } from './runner/hosts'

describe('the Infrastructure', () => {
  test('must create global nodes pool', async () => {
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

  test('must set up global nodes', async () => {
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
              topic: 'Cosmos integration',
            },
          },
        },
      },
    ])
  })

  test('must allow event communication', async () => {
    const events = await runOnEach([{}, {}], false, async (node) => {
      node.actyxOS.eventService.publish({
        eventDrafts: [EventDraft.make('the Infrastructure', node.name, 42)],
      })
      const events: Event[] = []
      await new Promise((res) => {
        const stop = node.actyxOS.eventService.subscribe({
          subscriptions: [{ streamSemantics: 'the Infrastructure' }],
          onEvent: (ev) => {
            events.push(ev)
            if (events.length === 2) {
              stop()
              res()
            }
          },
        })
      })
      return events
    })

    expect(events.flat().map((ev) => ev.payload)).toEqual([42, 42, 42, 42])

    const ev1 = events[0].map((ev) => ev.stream.streamName)
    ev1.sort()
    const ev2 = events[1].map((ev) => ev.stream.streamName)
    ev2.sort()

    expect(ev1).toEqual(allNodeNames().slice(0, 2))
    expect(ev2).toEqual(allNodeNames().slice(0, 2))
  })
})
