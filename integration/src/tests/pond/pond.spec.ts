import { Fish, FishId, Pond, Tag, Tags, Where } from '@actyx/pond'
import { Observable } from 'rxjs'
import { runConcurrentlyOnAll, withPond } from '../../infrastructure/hosts'

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

describe.skip('Pond', () => {
  // FIXME: Parameters should be so high that these tests run for minutes on CI.
  const eventsLinear = 100
  const eventsConcurrent = 20

  // Assert that events are always fed to Fish in the correct order on every node, at any time,
  // and also assert that all events reach all Fish eventually.
  test('ordering / time travel', async () => {
    const randomId = String(Math.random())

    const results = await runConcurrentlyOnAll<string[]>((nodes) => {
      const t = (pond: Pond) =>
        concurrentOrderingTest(nodes.length, eventsLinear, Tag(randomId), pond)
      return nodes.map((node) => withPond(node, t))
    })

    expect(results.length).toBeGreaterThan(1)

    const firstResult = results[0]
    expect(isSortedAsc(firstResult)).toBeTruthy()

    for (const res of results) {
      expect(res).toEqual(firstResult)
    }
  }, 180_000)

  // Assert that Fish only receive exactly those events that they are subscribed to,
  // and always in the proper order.
  test('event filter / subscription', async () => {
    const randomId = String(Math.random())
    const base = Tag(randomId)

    const allResults = await runConcurrentlyOnAll<string[][]>((nodes) => {
      const t = (pond: Pond) =>
        Promise.all([
          // Some different ways in which tags might be combined.
          // We run all of these in parallel, to assert their events are not accidentally mixed up.
          concurrentOrderingTest(nodes.length, eventsConcurrent, base.withId('/1'), pond),
          concurrentOrderingTest(nodes.length, eventsConcurrent, base.and('/2'), pond),
          concurrentOrderingTest(
            nodes.length,
            eventsConcurrent,
            Tag(':3:').and(base).and('::3'),
            pond,
          ),
          concurrentOrderingTest(
            nodes.length,
            eventsConcurrent,
            Tag('xxx').and(base.withId('___4___')),
            pond,
          ),
          concurrentOrderingTest(nodes.length, eventsConcurrent, Tag('5').withId(randomId), pond),
        ])
      return nodes.map((node) => withPond(node, t))
    })

    // Basically we need to `transpose` allResults.
    // But our fp-ts is too old, does not have transpose fn.
    for (const resNumber in allResults[0]) {
      const results = allResults.map((r) => r[resNumber])

      expect(results.length).toBeGreaterThan(1)

      const firstResult = results[0]
      expect(isSortedAsc(firstResult)).toBeTruthy()

      for (const res of results) {
        expect(res).toEqual(firstResult)
      }
    }
  })

  // Roughly assert causal consistency, by asserting that for every event we see in the Fish,
  // we have already seen at least as many events as the writer of that event.
  // FIXME currently skipped because we have no causal consistency guarantee.
  test.skip('sequencing / causal consistency', async () => {
    const randomId = String(Math.random())

    const results = await runConcurrentlyOnAll<string[]>((nodes) => {
      const t = sequenceCausalityTest(nodes.length, randomId)
      return nodes.map((node) => withPond(node, t))
    })

    for (const res of results) {
      expect(isSortedAsc(res)).toBeTruthy()
    }
  })
})

const randomTags = (prefix: string) => (c: number): Tags<never> => {
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
  eventsPerNode: number,
  streamTags: Tags<unknown>,
  pond: Pond,
): Promise<string[]> => {
  const where = padSubWithDummies(streamTags)

  const { nodeId } = pond.info()

  const f: Fish<string[], unknown> = {
    where,
    initialState: [],
    onEvent: (state, _event, metadata) => {
      state.push(metadata.eventId)
      return state
    },
    fishId: FishId.of('orderingtest', fishName(nodeId, where), 1),
  }

  const expectedSum = numNodes * eventsPerNode

  const state = new Promise<string[]>((resolve, reject) =>
    pond.observe(
      f,
      (state) => {
        if (state.length === expectedSum) {
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
  for (let i = 0; i < eventsPerNode; i++) {
    await pond.emit(emissionTags, { hello: 'world' }).toPromise()
    await Observable.timer(10 * Math.random()).toPromise()
  }

  return state
}

const sequenceCausalityTest = (numNodes: number, streamName: string) => async (
  pond: Pond,
): Promise<string[]> => {
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
      await Observable.timer(10 * Math.random()).toPromise()
    },
    (state) => state.length >= expectedSum,
  )

  const res = await state

  cancel()

  return res
}
