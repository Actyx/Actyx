import { Fish, FishId, Pond, Tag } from '@actyx/pond'
import { Observable } from 'rxjs'
import { runConcurrentlyOnAll, withPond } from '../infrastructure/hosts'

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
  test('ordering / time travel', async () => {
    const randomId = String(Math.random())

    const results = await runConcurrentlyOnAll<string[]>((nodes) => {
      const t = concurrentOrderingTest(nodes.length, randomId)
      return nodes.map((node) => withPond(node, t))
    })

    expect(results.length).toBeGreaterThan(1)

    const firstResult = results[0]
    expect(isSortedAsc(firstResult)).toBeTruthy()

    for (const res of results) {
      expect(res).toEqual(firstResult)
    }
  })

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

// Emit events while reading events from other nodes, too.
// Assert all intermediate observed states are properly sorted (time travel)
// and the final state (all events from all participants) is reached.
const concurrentOrderingTest = (numNodes: number, streamName: string) => async (
  pond: Pond,
): Promise<string[]> => {
  const where = Tag(streamName)

  const f: Fish<string[], unknown> = {
    where,
    initialState: [],
    onEvent: (state, _event, metadata) => {
      state.push(metadata.eventId)
      return state
    },
    fishId: FishId.of('orderingtest', 'test', 1),
  }

  const eventsPerNode = 1000

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

  for (let i = 0; i < eventsPerNode; i++) {
    await pond.emit(Tag('whatever').and(where), { hello: 'world' }).toPromise()
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

  const f: Fish<string[], Event> = {
    where: tags,
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
    fishId: FishId.of('seqtest', 'test', 1),
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

  const cancel = pond.keepRunning(
    f,
    async (state, enqueue) => {
      enqueue(tags, { numLocallyKnown: state.length })
      await Observable.timer(10 * Math.random()).toPromise()
    },
    (state) => state.length >= expectedSum,
  )

  const res = await state

  cancel()

  return res
}
