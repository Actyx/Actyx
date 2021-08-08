/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { chunksOf } from 'fp-ts/lib/Array'
import { Subject } from '../../node_modules/rxjs'
import {
  ActyxEvent,
  allEvents,
  EarliestQuery,
  EventChunk,
  EventOrder,
  EventsSortOrder,
  NodeId,
  Where,
} from '..'
import { SnapshotStore } from '../snapshotStore'
import { EventFnsFromEventStoreV2 } from './event-fns-impl'
import { EventStore } from './eventStore'
import { emitter, mkTimeline } from './testHelper'
import { Event, Events } from './types'

const assertPayloadsEqual = (actual: ActyxEvent[], expected: Events) =>
  expect(actual.map(x => x.payload)).toEqual(expected.map(x => x.payload))

const toOffsets = (eventsAscending: Events) => {
  const offsets: Record<string, number> = {}
  for (const ev of eventsAscending) {
    offsets[ev.stream] = ev.offset
  }
  return offsets
}

type PartialChunk = {
  events: Partial<ActyxEvent>[]
  lowerBound: EventChunk['lowerBound']
  upperBound: EventChunk['upperBound']
}

// Simulate chunking according to forward/reverse order, but keep everything in asc order,
// so that we have an easier time computing the expected offset maps.
const rawChunksAsc = (eventsAscending: Events, chunkSize: number, reverse?: boolean) => {
  const clientChunk = chunksOf<Event>(chunkSize)
  if (reverse) {
    // This is a bit complicated, basically we reverse all events, chunk once, chunk twice, reverse the indicidual chunks, reverse the list of chunks.
    // So we return events ascending, in ascending chunks, but chunked according to reverse logic.
    return clientChunk(eventsAscending.reverse())
      .map(x => x.reverse())
      .reverse()
  } else {
    return clientChunk(eventsAscending)
  }
}

const expectedChunks = (
  eventsAscending: Events,
  chunkSize: number,
  opts: {
    initialLowerBound?: Record<string, number>
    reverse?: boolean
  },
): PartialChunk[] => {
  // Even if opts.reverse is true, everything about these chunks is in ascending order. Just the size distribution is different.
  const chunksAscending = rawChunksAsc(eventsAscending, chunkSize, opts.reverse)

  const expected: PartialChunk[] = []
  let curLowerBound = opts.initialLowerBound || {}

  for (const chunk of chunksAscending) {
    const upperBound = {
      ...curLowerBound,
      ...toOffsets(chunk),
    }

    const contents = chunk.map(x => ({ payload: x.payload }))
    expected.push({
      events: opts.reverse ? contents.reverse() : contents,
      lowerBound: { ...curLowerBound },
      upperBound: { ...upperBound },
    })

    curLowerBound = upperBound
  }

  return opts.reverse ? expected.reverse() : expected
}

const buffer = () => {
  const values = new Subject()

  const cb = (val: unknown) => values.next(val)

  let finished = false

  const finishedCb = () => {
    finished = true
  }

  const expectResultEq = async (op: () => void, ...expected: unknown[]) => {
    const r = values
      .take(expected.length)
      .toArray()
      .toPromise()
    op()

    expect(await r).toEqual(expected)
  }

  const expectResultMatches = async (op: () => void, ...expected: unknown[]) => {
    const r = values
      .take(expected.length)
      .toArray()
      .toPromise()

    op()

    expect(await r).toMatchObject(expected)
  }

  const expectNoResult = async (op: () => void) => {
    const r = values.timeout(20).toPromise()

    op()

    await expect(r).rejects.toBeTruthy()
  }

  const assertFinished = async () => {
    await new Promise(res => setTimeout(res, 1))
    expect(finished).toBeTruthy()
  }

  return { cb, expectResultEq, expectResultMatches, expectNoResult, assertFinished, finishedCb }
}

const setup = () => {
  const srcA = emitter('A')
  const srcB = emitter('B')
  const srcC = emitter('C')
  const tl = mkTimeline(srcC(5), srcB(6), srcA(7), srcA(8), srcB(9), srcC(10))

  const store = EventStore.test()
  const fns = EventFnsFromEventStoreV2(NodeId.of('noop'), store, SnapshotStore.noop)

  return { store, fns, tl }
}

describe('EventFns', () => {
  it(`should find new events`, async () => {
    const { fns, tl, store } = setup()
    store.directlyPushEvents(tl.all)

    const res = await fns.queryAllKnown({ query: allEvents })

    assertPayloadsEqual(res.events, tl.all)
  })

  it(`should find new events incrementally`, async () => {
    const { fns, tl, store } = setup()
    const eventsAC = tl.of('A', 'C')

    store.directlyPushEvents(eventsAC)

    const res = await fns.queryAllKnown({ query: allEvents })

    assertPayloadsEqual(res.events, eventsAC)
    expect(res.upperBound).toEqual(toOffsets(eventsAC))

    // Add B's events
    const eventsB = tl.of('B')
    store.directlyPushEvents(eventsB)

    const res2 = await fns.queryAllKnown({
      query: allEvents,
      lowerBound: res.upperBound,
    })
    assertPayloadsEqual(res2.events, eventsB)
    expect(res2.upperBound).toEqual(toOffsets(tl.all))
  })

  describe('chunking', () => {
    it('should call onComplete even when there are no events', () => {
      const { fns } = setup()

      const { cb } = buffer()

      const res = new Promise(resolve => fns.queryAllKnownChunked({}, 15000, cb, resolve))

      return expect(res).resolves.toBeUndefined()
    })

    it('should call onComplete even when immediately canceled', () => {
      const { fns, store, tl } = setup()
      store.directlyPushEvents(tl.all)

      const { cb } = buffer()

      const res = new Promise(resolve => fns.queryAllKnownChunked({}, 15000, cb, resolve)())

      return expect(res).resolves.toBeUndefined()
    })

    const testChunking = async (chunkSize: number, order: EventsSortOrder) => {
      const { fns, tl, store } = setup()
      const events = tl.all
      store.directlyPushEvents(events)

      const expChunks = expectedChunks(events, chunkSize, { reverse: order === 'desc' })

      const { cb, expectResultMatches, finishedCb, assertFinished } = buffer()

      await expectResultMatches(
        () => fns.queryAllKnownChunked({ order }, chunkSize, cb, finishedCb),
        ...expChunks,
      )

      await assertFinished()
    }

    const testBoth = (description: string, testFn: (ord: EventsSortOrder) => Promise<void>) => {
      it(description + ' (ASC)', async () => await testFn(EventsSortOrder.Ascending))
      it(description + ' (DESC)', async () => await testFn(EventsSortOrder.Descending))
    }

    testBoth('should work with chunk size eq to store chunk size', async ord => {
      // We will get chunk sizes 4 - 2
      await testChunking(4, ord)
    })

    testBoth('should work with divisors of store chunk size', async ord => {
      // We will get chunk sizes 2 - 2 - 2
      await testChunking(2, ord)
    })

    testBoth('should work with non-divisors of store chunk size', async ord => {
      // We will get chunk sizes 3 - 1 - 2
      await testChunking(3, ord)
    })

    testBoth(
      'should work with chunk size larger than store (will not withhold events to fill chunks)',
      async ord => {
        // We will get chunk sizes 4 - 2
        await testChunking(5, ord)
      },
    )
  })

  describe('range queries', () => {
    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')
    const tl = mkTimeline(srcA(8), srcB(9), srcC(10))

    it('should omit unspecified sources', async () => {
      const { fns, store } = setup()
      store.directlyPushEvents(tl.all)

      for (const src of ['A', 'B', 'C']) {
        const evts = tl.of(src)
        const r = await fns.queryKnownRange({ upperBound: toOffsets(evts), query: allEvents })
        assertPayloadsEqual(r, evts)
      }

      const evtsAC = tl.of('A', 'C')
      const r = await fns.queryKnownRange({ upperBound: toOffsets(evtsAC), query: allEvents })
      assertPayloadsEqual(r, evtsAC)
    })
  })

  describe('subscription', () => {
    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')
    const tl = mkTimeline(srcA(8), srcB(9), srcC(10))

    it('should deliver events ASAP and with correct offset maps', async () => {
      const { fns, store } = setup()

      const { cb, expectResultMatches } = buffer()

      const cancel = fns.subscribeChunked({}, {}, cb)

      for (const src of ['A', 'B', 'C']) {
        const tlS = tl.of(src)
        const exp = expectedChunks(tlS, 3, {})

        await expectResultMatches(() => store.directlyPushEvents(tlS), ...exp)
      }

      cancel()
    })

    it('should deliver existing events', async () => {
      const { fns, store, tl } = setup()

      const { cb, expectResultMatches } = buffer()

      const tlAC = tl.of('A', 'C')
      store.directlyPushEvents(tlAC)

      // Strictly speaking, the store may interleave A and C in any order it likes. So we just test the offsets.
      await expectResultMatches(() => fns.subscribeChunked({}, {}, cb), {
        lowerBound: {},
        upperBound: toOffsets(tlAC),
      })

      const tlB = tl.of('B')
      const expB = expectedChunks(tlB, 5000, {})

      await expectResultMatches(() => store.directlyPushEvents(tlB), ...expB)

      store.close()
    })
  })

  describe('reduce unordered', () => {
    it('should start out with current result', async () => {
      const { fns, tl, store } = setup()
      store.directlyPushEvents(tl.all)

      const { cb, expectResultEq } = buffer()

      await expectResultEq(
        () =>
          fns.observeUnorderedReduce<number, number>(
            allEvents as Where<number>,
            (s, e) => s + e,
            0,
            cb,
          ),
        45,
      )

      store.close()
    })

    it('should update incrementally', async () => {
      const { fns, tl, store } = setup()
      store.directlyPushEvents(tl.of('A'))

      const { cb, expectResultEq } = buffer()

      await expectResultEq(
        () =>
          fns.observeUnorderedReduce<number, number>(
            allEvents as Where<number>,
            (s, e) => s + e,
            0,
            cb,
          ),
        15,
      )

      await expectResultEq(() => store.directlyPushEvents(tl.of('B')), 30)

      await expectResultEq(() => store.directlyPushEvents(tl.of('B')), 45)

      store.close()
    })
  })

  describe('observe earliest / latest', () => {
    describe('with either order', () => {
      const testBoth = (
        description: string,
        testFn: (q: EarliestQuery<unknown>) => Promise<void>,
      ) => {
        it(description + ' (lamport)', async () => await testFn({ query: allEvents }))
        it(
          description + ' (timestamp)',
          async () => await testFn({ query: allEvents, eventOrder: EventOrder.Timestamp }),
        )
      }

      testBoth('should wait for 1st event to be found', async q => {
        const { store, fns, tl } = setup()

        let called = false
        const earliest = new Promise(resolve => fns.observeEarliest(q, resolve)).then(x => {
          called = true
          return x
        })

        const latest = new Promise(resolve => fns.observeLatest(q, resolve)).then(x => {
          called = true
          return x
        })

        // Wait some time
        await new Promise(r => setTimeout(r, 5))
        expect(called).toBeFalsy() // Not called yet

        store.directlyPushEvents(tl.all)

        await expect(earliest).resolves.toEqual(5)
        await expect(latest).resolves.toEqual(10)

        store.close()
      })

      testBoth('should directly deliver known result', async q => {
        const { store, fns, tl } = setup()
        store.directlyPushEvents(tl.all)

        const earliest = await new Promise(resolve => fns.observeEarliest(q, resolve))
        expect(earliest).toEqual(5)

        const latest = await new Promise(resolve => fns.observeLatest(q, resolve))
        expect(latest).toEqual(10)

        store.close()
      })

      testBoth('should update earliest when new information becomes known', async q => {
        const { store, fns, tl } = setup()
        store.directlyPushEvents(tl.of('A'))

        const { cb, expectResultEq } = buffer()

        await expectResultEq(() => fns.observeEarliest(q, cb), 7)

        await expectResultEq(() => store.directlyPushEvents(tl.of('B')), 6)

        await expectResultEq(() => store.directlyPushEvents(tl.of('C')), 5)
        store.close()
      })

      testBoth('should not update earliest when it doesnt change', async q => {
        const { store, fns, tl } = setup()
        store.directlyPushEvents(tl.of('C'))

        const { cb, expectResultEq, expectNoResult } = buffer()

        await expectResultEq(() => fns.observeEarliest(q, cb), 5)

        await expectNoResult(() => store.directlyPushEvents(tl.of('B')))

        await expectNoResult(() => store.directlyPushEvents(tl.of('A')))
        store.close()
      })

      testBoth('should update latest when new information becomes known', async q => {
        const { store, fns, tl } = setup()
        store.directlyPushEvents(tl.of('A'))

        const { cb, expectResultEq } = buffer()

        await expectResultEq(() => fns.observeLatest(q, cb), 8)

        await expectResultEq(() => store.directlyPushEvents(tl.of('B')), 9)

        await expectResultEq(() => store.directlyPushEvents(tl.of('C')), 10)
        store.close()
      })

      testBoth('should not update latest when it doesnt change', async q => {
        const { store, fns, tl } = setup()
        store.directlyPushEvents(tl.of('C'))

        const { cb, expectResultEq, expectNoResult } = buffer()

        await expectResultEq(() => fns.observeLatest(q, cb), 10)

        await expectNoResult(() => store.directlyPushEvents(tl.of('B')))

        await expectNoResult(() => store.directlyPushEvents(tl.of('A')))
        store.close()
      })
    })

    describe('with timestamp order', () => {
      const query = { query: allEvents, eventOrder: EventOrder.Timestamp }

      it('should find highest timestamp even with jumping clock', async () => {
        const { store, fns, tl } = setup()

        // Make very large...
        tl.all[2].timestamp *= 1000

        store.directlyPushEvents(tl.all)

        const latest = await new Promise(resolve => fns.observeLatest(query, resolve))
        expect(latest).toEqual(7)
        store.close()
      })

      it('should find lowest timestamp even with jumping clock', async () => {
        const { store, fns, tl } = setup()

        tl.all[0].timestamp = 110
        tl.all[2].timestamp = 10

        store.directlyPushEvents(tl.all)

        const earliest = await new Promise(resolve => fns.observeEarliest(query, resolve))
        expect(earliest).toEqual(7)
        store.close()
      })
    })
  })

  describe('find best match', () => {
    // We look for the event with value closes to 5.8...
    const distance = (e: ActyxEvent<number>) => Math.abs(5.8 - e.payload)

    const shouldReplace = (candidate: ActyxEvent<number>, cur: ActyxEvent<number>) => {
      return distance(candidate) < distance(cur)
    }

    it('should start out with current result', async () => {
      const { fns, tl, store } = setup()
      store.directlyPushEvents(tl.all)

      const { cb, expectResultEq } = buffer()

      await expectResultEq(
        () => fns.observeBestMatch<number>(allEvents as Where<number>, shouldReplace, cb),
        6,
      )
      store.close()
    })

    it('should update incrementally', async () => {
      const { fns, tl, store } = setup()
      store.directlyPushEvents(tl.of('A'))

      const { cb, expectResultEq } = buffer()

      await expectResultEq(
        () => fns.observeBestMatch<number>(allEvents as Where<number>, shouldReplace, cb),
        7,
      )

      await expectResultEq(() => store.directlyPushEvents(tl.of('C')), 5)

      await expectResultEq(() => store.directlyPushEvents(tl.of('B')), 6)
      store.close()
    })
  })
})
