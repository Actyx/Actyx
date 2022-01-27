/**
 * @jest-environment ./dist/jest/environment
 */
import { Fish, FishId, Pond, Tag, Tags, Where } from '@actyx/pond'
import { Observable, lastValueFrom, timer } from 'rxjs'
import { filter, take } from 'rxjs/operators'
import { SettingsInput } from '../../cli/exec'
import { trialManifest } from '../../http-client'
import { runConcurrentlyOnAll, runWithNewProcess, withPond } from '../../infrastructure/hosts'
import { randomString } from '../../util'

const isSortedAsc = (data: string[]): boolean => {
  if (data.length < 2) {
    return true
  }

  let d = data[0]
  for (const element of data) {
    if (element < d) {
      return false
    }

    d = element
  }

  return true
}

describe('Pond', () => {
  // FIXME: Parameters should be so high that these tests run for minutes on CI.
  const roundsPerNodeLinear = 64
  const roundsPerNodeConcurrent = roundsPerNodeLinear / 4

  // Assert that events are always fed to Fish in the correct order on every node, at any time,
  // and also assert that all events reach all Fish eventually.
  test('ordering / time travel', async () => {
    const randomId = randomString()

    const results = await runConcurrentlyOnAll<string[]>((nodes) => {
      const t = (pond: Pond, nodeName: string) =>
        concurrentOrderingTest(nodes.length, roundsPerNodeLinear, Tag(randomId), pond, nodeName)
      return nodes.map((node) => withPond(node, t))
    })

    if (results.length === 1) {
      console.warn('Pond spec ran with just 1 node')
    }

    const firstResult = results[0]
    expect(isSortedAsc(firstResult)).toBeTruthy()

    for (const res of results) {
      expect(res).toEqual(firstResult)
    }
  }, 600_000)

  // Assert that Fish only receive exactly those events that they are subscribed to,
  // and always in the proper order.
  test('event filter / subscription', async () => {
    const randomId = randomString()
    const base = Tag(randomId)

    const allResults = await runConcurrentlyOnAll<string[][]>((nodes) => {
      const t = (pond: Pond, nodeName: string) =>
        Promise.all([
          // Some different ways in which tags might be combined.
          // We run all of these in parallel, to assert their events are not accidentally mixed up.
          concurrentOrderingTest(
            nodes.length,
            roundsPerNodeConcurrent,
            base.withId('/1'),
            pond,
            nodeName,
          ),
          concurrentOrderingTest(
            nodes.length,
            roundsPerNodeConcurrent,
            base.and('/2'),
            pond,
            nodeName,
          ),
          concurrentOrderingTest(
            nodes.length,
            roundsPerNodeConcurrent,
            Tag(':3:').and(base).and('::3'),
            pond,
            nodeName,
          ),
          concurrentOrderingTest(
            nodes.length,
            roundsPerNodeConcurrent,
            Tag('xxx').and(base.withId('___4___')),
            pond,
            nodeName,
          ),
          concurrentOrderingTest(
            nodes.length,
            roundsPerNodeConcurrent,
            Tag('5').withId(randomId),
            pond,
            nodeName,
          ),
        ])
      return nodes.map((node) => withPond(node, t))
    })

    // Basically we need to `transpose` allResults.
    // But our fp-ts is too old, does not have transpose fn.
    for (const resNumber in allResults[0]) {
      const results = allResults.map((r) => r[resNumber])

      if (results.length === 1) {
        console.warn('Pond spec ran with just 1 node')
      }

      const firstResult = results[0]
      expect(isSortedAsc(firstResult)).toBeTruthy()

      for (const res of results) {
        expect(res).toEqual(firstResult)
      }
    }
  }, 600_000)

  // Roughly assert causal consistency, by asserting that for every event we see in the Fish,
  // we have already seen at least as many events as the writer of that event.
  // FIXME currently skipped because we have no causal consistency guarantee.
  test.skip('sequencing / causal consistency', async () => {
    const randomId = randomString()

    const results = await runConcurrentlyOnAll<string[]>((nodes) => {
      const t = sequenceCausalityTest(nodes.length, randomId)
      return nodes.map((node) => withPond(node, t))
    })

    for (const res of results) {
      expect(isSortedAsc(res)).toBeTruthy()
    }
  }, 300_000)

  test('automatically reconnect Fish (no error propagation to user) if automaticReconnect=true', async () =>
    runWithNewProcess(async (node) => {
      const pond = await Pond.of(
        trialManifest,
        {
          actyxPort: node._private.apiPort,
          automaticReconnect: true,
        },
        {},
      )

      const tag = Tag<number>('numbers').withId(randomString())

      type State = {
        numEvents: number
        lastEvent: number
      }

      const eventCounterFish: Fish<State, number> = {
        where: tag,
        initialState: {
          numEvents: 0,
          lastEvent: -1,
        },
        onEvent: (state, event) => ({
          lastEvent: event,
          numEvents: state.numEvents + 1,
        }),
        fishId: FishId.of('ttt', 't', 1),
      }
      try {
        await pond.publish(tag.apply(1))

        const states = new Observable<State>((o) =>
          pond.observe(eventCounterFish, (x) => o.next(x)),
        )
        const firstState = await lastValueFrom(states.pipe(take(1)))
        expect(firstState).toEqual({
          lastEvent: 1,
          numEvents: 1,
        })

        // Topic change causes WS to be closed. We cannot use `powerCycle` because that gives new port numbers...
        await node.ax.settings.set('/swarm/topic', SettingsInput.FromValue('A different topic'))

        let numErrs = 0
        for (;;) {
          try {
            await pond.publish(tag.apply(5))
            break
          } catch (_err) {
            numErrs += 1
          }
        }
        // We should get 0-1 errors depending on the reconnect timing
        expect(numErrs).toBeLessThan(2)

        const e = lastValueFrom(
          states.pipe(
            filter((x) => x.lastEvent === 5),
            take(1),
          ),
        )
        expect(e).resolves.toEqual({
          // We switched to the other topic and lost the events from the old one...
          numEvents: 1,
          lastEvent: 5,
        })
      } finally {
        pond.dispose()
      }
    }))
})

const randomTags =
  (prefix: string) =>
  (c: number): Tags<never> => {
    return Tag(prefix)
      .withId(String(c))
      .and(Tag(c + 'ok'))
  }

const padSubWithDummies = <E>(where: Where<E>): Where<E> => {
  const rt = randomTags('in')

  for (;;) {
    const c = Math.random()
    if (c < 0.4) {
      where = where.or(rt(c)) as Where<E>
    } else if (c < 0.8) {
      where = rt(c).or(where)
    } else {
      break
    }
  }

  return where
}

const padEmitWithDummies = <E>(tags: Tags<E>): Tags<E> => {
  const rt = randomTags('out')

  for (;;) {
    const c = Math.random()
    if (c < 0.4) {
      tags = tags.and(rt(c))
    } else if (c < 0.8) {
      tags = rt(c).and(tags)
    } else {
      break
    }
  }

  return tags
}

const fishName = (source: string, subs: Where<unknown>) =>
  `source=${source}, subs=${subs.toString()}`

// Emit events while reading events from other nodes, too.
// Assert all intermediate observed states are properly sorted (time travel)
// and the final state (all events from all participants) is reached.
const concurrentOrderingTest = async (
  numNodes: number,
  roundsPerNode: number,
  streamTags: Tags<unknown>,
  pond: Pond,
  nodeName = 'not-given',
): Promise<string[]> => {
  const where = padSubWithDummies(streamTags)

  const { nodeId } = pond.info()

  const start = Date.now()

  const f: Fish<string[], unknown> = {
    where,
    initialState: [],
    onEvent: (state, _event, metadata) => {
      state.push(metadata.eventId)
      return state
    },
    fishId: FishId.of('orderingtest', fishName(nodeId, where), 1),
  }

  const expectedSum = numNodes * roundsPerNode * 4 // Each round emits 4 events

  const state = new Promise<string[]>((resolve, reject) =>
    pond.observe(
      f,
      (state) => {
        if (state.length === expectedSum) {
          const end = Date.now()
          const took = String((end - start) / 1_000.0)
          process.stdout.write(
            '  pond.spec.ts: ' +
              [
                nodeName,
                '[',
                where.toString(),
                '] finished in',
                took,
                's, found:',
                expectedSum,
              ].join(' ') +
              '\n',
          )
          resolve(state)
        }

        // All intermediate results should be sorted
        if (!isSortedAsc(state)) {
          reject(new Error('incorrect sorting: ' + JSON.stringify(state)))
        }
      },
      reject,
    ),
  )

  const emissionTags = padEmitWithDummies(streamTags)
  const e = pond.events()

  const emissionStart = Date.now()
  for (let i = 0; i < roundsPerNode; i++) {
    await pond.emit(emissionTags, { hello: 'world0' }).toPromise()
    await pond.emit(emissionTags, { hello: 'world1' }).toPromise()
    await e.emit(emissionTags.apply({ hello: 'world2' }, { hello: 'world3' })).toPromise()
    await new Promise((res) => setTimeout(res, 10 * Math.random())) // Sleep 0-10ms
  }
  const emissionDuration = Date.now() - emissionStart
  process.stdout.write(
    '  pond.spec.ts: ' +
      [
        nodeName,
        'emission of',
        roundsPerNode * 4,
        'events took',
        String(emissionDuration / 1_000.0),
      ].join(' ') +
      '\n',
  )

  return state
}

const sequenceCausalityTest =
  (numNodes: number, streamName: string) =>
  async (pond: Pond): Promise<string[]> => {
    type Event = {
      numLocallyKnown: number
    }

    const tags = Tag(streamName).and(Tag<Event>('seqtest'))

    const where = padSubWithDummies(tags)

    const { nodeId } = pond.info()

    const f: Fish<string[], Event> = {
      where,
      initialState: [],
      onEvent: (state, event, metadata) => {
        if (state.length < event.numLocallyKnown) {
          throw new Error(
            'We know less events than event sender! Us:' +
              state.length +
              ' vs. sender:' +
              event.numLocallyKnown,
          )
        }

        state.push(metadata.eventId)
        return state
      },
      fishId: FishId.of('seqtest', fishName(nodeId, where), 1),
    }

    const eventsPerNode = 1000

    const expectedSum = numNodes * eventsPerNode

    const state = new Promise<string[]>((resolve, reject) =>
      pond.observe(
        f,
        (state) => {
          if (state.length >= expectedSum) {
            resolve(state)
          }

          // All intermediate results should be sorted
          if (!isSortedAsc(state)) {
            reject(new Error('incorrect sorting: ' + JSON.stringify(state)))
          }
        },
        reject,
      ),
    )

    const emissionTags = padEmitWithDummies(tags)
    const cancel = pond.keepRunning(
      f,
      async (state, enqueue) => {
        enqueue(emissionTags, { numLocallyKnown: state.length })
        await timer(10 * Math.random()).toPromise()
      },
      (state) => state.length >= expectedSum,
    )

    const res = await state

    cancel()

    return res
  }
