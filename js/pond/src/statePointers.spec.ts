/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { getRecentPointers, StatePointers } from './statePointers'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { EnvelopeFromStore } from './store/util'
import { StatePointer, StateWithProvenance, Timestamp } from './types'

const neverSnapshotScheduler: SnapshotScheduler = {
  minEventsForSnapshot: 1,
  getSnapshotLevels: (_, _ts) => [],
  isEligibleForStorage: (_pending, _latest) => {
    return true
  },
}

const mockEvent = (source: string) =>
  (({ source: { sourceId: source } } as unknown) as EnvelopeFromStore<undefined>)
const mockState = ('mock-state' as unknown) as StateWithProvenance<undefined>

const uniqTagged = (
  i: number,
  localSnap = false,
  source = 'some-source',
): StatePointer<undefined, undefined> => ({
  i,
  tag: Math.random().toString(),
  persistAsLocalSnapshot: localSnap,
  state: mockState,
  finalIncludedEvent: mockEvent(source),
})

describe('State Pointers', () => {
  describe('scheduling', () => {
    const TEST_RECENT_WINDOW = 32
    const TEST_RECENT_SPACING = 4

    it(`should request a pointer for every source tip, and another one every ${TEST_RECENT_SPACING} events starting at length-1`, () => {
      const envelopes = [
        mockEvent('A'),
        mockEvent('A'),
        mockEvent('A'),
        mockEvent('A'),
        mockEvent('B'),
        mockEvent('C'),
      ]

      const sp = new StatePointers<undefined, undefined>(
        neverSnapshotScheduler,
        TEST_RECENT_WINDOW,
        TEST_RECENT_SPACING,
      )

      const toCache = sp.getStatesToCache(Timestamp.of(0), envelopes)

      expect(toCache.map(x => x.i)).toEqual([3, 4, 4, 5])
    })

    it('should have configurable recent state behaviour', () => {
      const envelope = mockEvent('A')
      const envelopes = new Array(52)
      envelopes.fill(envelope)

      // One state cached every 5 events, for the most recent 20 events
      const sp = new StatePointers<undefined, undefined>(neverSnapshotScheduler, 20, 5)

      const toCache = sp.getStatesToCache(Timestamp.of(0), envelopes)

      expect(toCache.map(x => x.i)).toEqual([35, 40, 45, 50, 51])
      // Assert all indices are unique
      expect(new Set(toCache.map(x => x.tag)).size).toEqual(toCache.length)
    })

    it(`should request a pointer in front, and one every ${TEST_RECENT_SPACING} events for the front ${TEST_RECENT_WINDOW} starting at the second foremost`, () => {
      const envelope = mockEvent('A')
      const envelopes = new Array(50)
      envelopes.fill(envelope)

      const sp = new StatePointers<undefined, undefined>(
        neverSnapshotScheduler,
        TEST_RECENT_WINDOW,
        TEST_RECENT_SPACING,
      )

      const toCache = sp.getStatesToCache(Timestamp.of(0), envelopes)

      expect(toCache.map(x => x.i)).toEqual([20, 24, 28, 32, 36, 40, 44, 48, 49])
      // Assert all indices are unique
      expect(new Set(toCache.map(x => x.tag)).size).toEqual(toCache.length)
    })

    it('should not request pointers earlier than its latest known', () => {
      const envelope = mockEvent('A')
      const envelopes = new Array(50)
      envelopes.fill(envelope)
      // Another source-tip in the past
      envelopes[35] = mockEvent('B')

      const sp = new StatePointers<undefined, undefined>(
        neverSnapshotScheduler,
        TEST_RECENT_WINDOW,
        TEST_RECENT_SPACING,
      )
      sp.addPopulatedPointers([uniqTagged(45)])

      const toCache = sp.getStatesToCache(Timestamp.of(0), envelopes)

      expect(toCache.map(x => x.i)).toEqual([48, 49])
    })

    it('should gracefully handle buffer length 0', () => {
      const sp = new StatePointers<undefined, undefined>(
        neverSnapshotScheduler,
        TEST_RECENT_WINDOW,
        TEST_RECENT_SPACING,
      )

      const toCache = sp.getStatesToCache(0, [])
      expect(toCache).toEqual([])
    })
  })

  describe('updating', () => {
    const ptrAt5 = uniqTagged(5)
    const ptrAt10 = uniqTagged(10)
    const ptrAt11 = uniqTagged(11, true)
    const ptrAt12 = uniqTagged(12)

    const setup = () => {
      // Pointers are mutable, so we need to start each test with a set of fresh ones
      const pointersCopy = [ptrAt5, ptrAt10, ptrAt11, ptrAt12].map(x => Object.assign({}, x))

      const sp = new StatePointers<undefined, undefined>(neverSnapshotScheduler)
      sp.addPopulatedPointers(pointersCopy)

      const expectLatest = (expected: StatePointer<undefined, undefined> | undefined) =>
        expect(sp.latestStored().toUndefined()).toEqual(expected)
      return { sp, expectLatest }
    }

    it('should invalidate everything above the number passed to invalidateDownTo', () => {
      const { sp, expectLatest } = setup()

      expectLatest(ptrAt12)

      sp.invalidateDownTo(11)
      expectLatest(ptrAt11)

      sp.invalidateDownTo(10)
      expectLatest(ptrAt10)

      sp.invalidateDownTo(3)
      expectLatest(undefined)
    })

    it('should shift pointers at shiftBack', () => {
      const { sp, expectLatest } = setup()

      sp.shiftBack(5)
      expectLatest({ ...ptrAt12, i: 7 })

      sp.shiftBack(7)
      expectLatest({ ...ptrAt12, i: 0 })

      sp.shiftBack(1)
      expectLatest(undefined)
    })

    it('should keep track of qualified snapshots', () => {
      const { sp, expectLatest } = setup()

      expect(sp.getSnapshotsToPersist()).toEqual([ptrAt11])

      sp.shiftBack(12)
      expectLatest({ ...ptrAt12, i: 0 })

      expect(sp.getSnapshotsToPersist()).toEqual([])
    })
  })

  describe('delayed qualification', () => {
    it('should emit local snapshots to persist after they are greenlit by the scheduler', () => {
      const snapshot = uniqTagged(11, true)

      let emit = false
      const mockScheduler: SnapshotScheduler = {
        minEventsForSnapshot: 1,
        getSnapshotLevels: (_, _ts) => [],
        isEligibleForStorage: (_pending, _latest) => {
          return emit
        },
      }

      const sp = new StatePointers<undefined, undefined>(mockScheduler)
      sp.addPopulatedPointers([snapshot])

      expect(sp.getSnapshotsToPersist()).toEqual([])

      emit = true

      // Need to add something in order to requalify local snapshots
      const n = uniqTagged(20)
      sp.addPopulatedPointers([n])
      expect(sp.latestStored().toUndefined()).toEqual(n)

      expect(sp.getSnapshotsToPersist()).toEqual([snapshot])
    })

    it('should migrate instead of replacing same-tagged, if applicable', () => {
      const snapshotOld = uniqTagged(11, true, 'A')
      const snapshotNew = {
        ...uniqTagged(20, true, 'B'),
        tag: snapshotOld.tag, // Same tag, so we would overwrite
      }

      let emit = false
      const mockScheduler: SnapshotScheduler = {
        minEventsForSnapshot: 1,
        getSnapshotLevels: (_, _ts) => [],
        isEligibleForStorage: (pending, _latest) => {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          return emit && (pending as any).source.sourceId === 'A'
        },
      }

      const sp = new StatePointers<undefined, undefined>(mockScheduler)
      sp.addPopulatedPointers([snapshotOld])

      expect(sp.getSnapshotsToPersist()).toEqual([])
      expect(sp.latestStored().toUndefined()).toEqual(snapshotOld)

      emit = true

      // Add with same tag
      sp.addPopulatedPointers([snapshotNew])
      // Make sure it is not outright rejected
      expect(sp.latestStored().toUndefined()).toEqual(snapshotNew)
      // ... but the old one has been remembered and moved to to-persist.
      expect(sp.getSnapshotsToPersist()).toEqual([snapshotOld])
    })
  })

  describe('getRecentPointers', () => {
    const gr = getRecentPointers(30, 6)

    it('should emit up to recent/spacing unique tagged indices, and roll over oldest to newest', () => {
      expect(gr(0, 101, -1)).toMatchObject([
        { tag: 'mod1', i: 96 },
        { tag: 'mod0', i: 90 },
        { tag: 'mod4', i: 84 },
        { tag: 'mod3', i: 78 },
        { tag: 'mod2', i: 72 },
      ])

      // mod2 and mod3 get recycled with the next extension of the event buffer
      expect(gr(0, 110, 101)).toMatchObject([{ tag: 'mod3', i: 108 }, { tag: 'mod2', i: 102 }])
    })

    it('should respect the limit', () => {
      expect(gr(0, 100, 90)).toMatchObject([{ tag: 'mod1', i: 96 }])
      expect(gr(0, 100, 88)).toMatchObject([{ tag: 'mod1', i: 96 }, { tag: 'mod0', i: 90 }])
    })

    it('should steer clear of 0', () => {
      expect(gr(0, 12, -1)).toMatchObject([{ tag: 'mod1', i: 6 }])
    })

    it('should steer clear of the tip', () => {
      // Length=13 means i=12 is the tip, but we do not want to store that, as it would duplicate a source-tip.
      expect(gr(0, 13, -1)).toMatchObject([{ tag: 'mod1', i: 6 }])
    })

    it('should keep stable when a cycleStart is passed', () => {
      let bufferLen = 400
      let cycleStart = 1

      type HasTag = { tag: string }
      let oldTags: HasTag[] = [
        {
          // i: 395,
          tag: 'mod1',
        },
        {
          // i: 389,
          tag: 'mod0',
        },
        {
          // i: 383,
          tag: 'mod4',
        },
        {
          // i: 377,
          tag: 'mod3',
        },
        {
          // i: 371,
          tag: 'mod2',
        },
      ]

      // Going lower than 30 we start to lose some pointers, of course.
      while (bufferLen > 30) {
        const newPtrs = gr(cycleStart, bufferLen, -1)

        expect(newPtrs).toMatchObject(oldTags)

        oldTags = newPtrs.map(t => ({
          tag: t.tag,
        }))

        const shift = Math.floor(Math.random() * 40)

        cycleStart += shift
        bufferLen -= shift
      }
    })

    it('should gracefully decline event buffer length 1 and 0', () => {
      const f = (a: number, b: number) => getRecentPointers(10, 1)(0, a, b)

      expect(f(0, -1)).toEqual([])
      expect(f(1, -1)).toEqual([])
      expect(f(2, -1)).toMatchObject([{ tag: 'mod1', i: 1 }])
      expect(f(3, -1)).toMatchObject([{ tag: 'mod2', i: 2 }, { tag: 'mod1', i: 1 }])
    })

    it('should not freak out if window size is not a full multiple of spacing', () => {
      const unevenGr = getRecentPointers(10, 3)

      expect(unevenGr(0, 50, -1)).toMatchObject([
        { tag: 'mod1', i: 48 },
        { tag: 'mod0', i: 45 },
        { tag: 'mod2', i: 42 },
      ])
    })
  })
})
