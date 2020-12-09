/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { Caching, Fish, FishId, Milliseconds, Pond, Tag } from '.'

type InitFish = {
  readonly fishSpecificTag: string
}

const seedEvents = Tag<InitFish>('init')

const makeMakeFish = (overrides?: (f: InitFish) => Partial<Fish<FishState, unknown>>) => (
  f: InitFish,
): Fish<FishState, unknown> => {
  const myTag = f.fishSpecificTag

  const upper = overrides ? overrides(f) : {}

  return {
    where: Tag(myTag),
    initialState: { myTag, numEvents: 0 },
    fishId: FishId.of('hello', myTag, 1),
    onEvent: (state, _event) => ({ ...state, numEvents: state.numEvents + 1 }),
    ...upper,
  }
}

const initFish = (pond: Pond, ...tags: unknown[]) => {
  for (const fishSpecificTag of tags) {
    pond.emit(seedEvents, { fishSpecificTag: String(fishSpecificTag) })
  }
}

type FishState = {
  readonly myTag: string
  readonly numEvents: number
}

const expectFishWithEvents = async (
  pond: Pond,
  makeFish: (f: InitFish) => Fish<FishState, unknown>,
  expectations: Record<string, number>,
) => {
  const states = await new Observable<FishState[]>(o =>
    pond.observeAll(
      seedEvents,
      makeFish,
      {
        // Randomly enable or disable caching - should make no difference
        caching: Math.random() > 0.5 ? Caching.inProcess('test') : undefined,
      },
      x => o.next(x),
    ),
  )
    .debounceTime(0)
    .first()
    .toPromise()

  const actual: Record<string, number> = {}
  for (const { myTag, numEvents } of states) {
    const exists = actual[myTag] !== undefined
    if (exists) throw new Error('duplicate fish: ' + myTag)
    actual[myTag] = numEvents
  }

  expect(actual).toMatchObject(expectations)
}

describe('Pond.observeAll', () => {
  it('should create all fish identified by seedEvents', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    initFish(pond, 1, 2, 3)

    await expectFishWithEvents(pond, makeFish, {
      1: 0,
      2: 0,
      3: 0,
    })

    pond.dispose()
  })

  it('should update all fish identified by seedEvents', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish((f: InitFish) => ({
      where: Tag(f.fishSpecificTag).or(seedEvents),
    }))

    initFish(pond, 1, 2, 3)

    // Every Fish has read all 3 InitFish events
    await expectFishWithEvents(pond, makeFish, {
      1: 3,
      2: 3,
      3: 3,
    })

    pond.dispose()
  })

  it('should return an empty array if no Fish are found', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    await expectFishWithEvents(pond, makeFish, {})

    pond.dispose()
  })

  it('should onboard new fish ad-hoc', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    initFish(pond, 1, 2)

    await expectFishWithEvents(pond, makeFish, {
      1: 0,
      2: 0,
    })

    // Assert we do not mind double-initialisation
    initFish(pond, 2, 3, 5)

    await expectFishWithEvents(pond, makeFish, {
      1: 0,
      2: 0,
      3: 0,
      5: 0,
    })

    pond.dispose()
  })

  it('should still de-duplicate Fish based on FishId', async () => {
    const pond = Pond.test()

    // Make the same fish from every event
    const makeFish = makeMakeFish(_f => ({
      fishId: FishId.of('same', 'same', 1),
      where: seedEvents,
    }))

    // Latest writer wins -- users should take care to actually make the same Fish,
    // unlike what we have done here, in order to test the implementation.
    initFish(pond, 'even if we init more than once', 'just one')

    await expectFishWithEvents(pond, makeFish, {
      'just one': 2,
    })

    pond.dispose()
  })

  it('should deliver personal events', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    initFish(pond, 1, 2, 3)

    pond.emit(Tag('1'), 'whatever')
    pond.emit(Tag('1'), 'whatever2')
    pond.emit(Tag('2'), 'whatever')

    await expectFishWithEvents(pond, makeFish, {
      1: 2,
      2: 1,
      3: 0,
    })

    pond.dispose()
  })

  it('should deliver events older than the seed event', async () => {
    const pond = Pond.test()

    pond.emit(Tag('1'), 'whatever')
    pond.emit(Tag('1'), 'whatever2')

    const makeFish = makeMakeFish()

    initFish(pond, 1, 2)

    await expectFishWithEvents(pond, makeFish, {
      1: 2,
      2: 0,
    })

    pond.dispose()
  })

  it('should just ignore events where makeFish returns undefined', async () => {
    const pond = Pond.test()

    const maybeMakeFish = (f: InitFish): Fish<FishState, unknown> => {
      if (f.fishSpecificTag.length === 1) {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        return undefined!
      }

      return makeMakeFish()(f)
    }

    initFish(pond, 1, 30, 2, 3, 20, 30)

    // All numbers with length=1 are ignored..
    await expectFishWithEvents(pond, maybeMakeFish, {
      20: 0,
      30: 0,
    })

    pond.dispose()
  })

  // FIXME: Too flaky on CI
  it.skip('should remove Fish from the set based on opts.expireAfterFirst', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    initFish(pond, 1, 2, 3)

    // Sleep 20 ms -> trigger expiry
    await Observable.timer(20).toPromise()

    initFish(pond, 2, 5)

    const states = new Observable<FishState[]>(o =>
      pond.observeAll(seedEvents, makeFish, { expireAfterSeed: Milliseconds.of(4) }, x =>
        o.next(x),
      ),
    )
      .debounceTime(0)
      .first()
      .toPromise()

    // 2 got another seed event, so it’s included -- the others are dropped
    await expectFishWithEvents(pond, makeFish, {
      2: 0,
      5: 0,
    })

    await expect(states).resolves.toMatchObject([{ myTag: '2' }, { myTag: '5' }])

    pond.dispose()
  })

  it('should not emit when state has not changed (no new Fish and no live Fish changed)', async () => {
    const pond = Pond.test()

    const makeFish = makeMakeFish()

    let cbInvoked = 0
    const assertInvocations = async (expected: number) => {
      // yield
      await Observable.timer(0).toPromise()
      expect(cbInvoked).toEqual(expected)
    }

    const cancel = pond.observeAll(
      seedEvents,
      makeFish,
      {
        caching: Caching.inProcess('test'),
      },
      _x => (cbInvoked += 1),
    )
    // Immediately invoked with empty array
    await assertInvocations(1)

    initFish(pond, 1, 2)
    await assertInvocations(2)

    initFish(pond, 1)
    await assertInvocations(2)

    // However, hitting the cache should immediately supply the last value, rather than wait for any change
    const cancel2 = pond.observeAll(
      seedEvents,
      makeFish,
      {
        caching: Caching.inProcess('test'),
      },
      _x => (cbInvoked += 1),
    )
    await assertInvocations(3)

    cancel()
    cancel2()
    pond.dispose()
  })
})

describe('Pond.observeOne', () => {
  const seedEvent = seedEvents

  const readState = async (pond: Pond, makeFish: (f: InitFish) => Fish<FishState, unknown>) =>
    new Observable<FishState>(o =>
      pond.observeOne(seedEvent, makeFish, x => o.next(x), err => o.error(err)),
    )
      .debounceTime(0)
      .first()
      .toPromise()

  it('should create the Fish from seedEvent', async () => {
    const pond = Pond.test()

    pond.emit(Tag('1'), '.')

    const makeFish = makeMakeFish()

    initFish(pond, 1)
    pond.emit(Tag('1'), '.')
    pond.emit(Tag('1'), '.')
    pond.emit(Tag('1'), '.')

    await expect(readState(pond, makeFish)).resolves.toMatchObject({ myTag: '1', numEvents: 4 })

    pond.dispose()
  })

  it('should allow multiple seedEvent, choosing any of them', async () => {
    const pond = Pond.test()
    const makeFish = makeMakeFish(() => ({
      where: seedEvent,
    }))

    initFish(pond, 4, 4, 4)

    await expect(readState(pond, makeFish)).resolves.toMatchObject({ myTag: '4', numEvents: 3 })

    pond.dispose()
  })

  it('should also propagate errors', async () => {
    let reported: FishId | null = null
    const pond = Pond.test({
      fishErrorReporter: (_, fishId) => {
        reported = fishId
      },
    })
    const makeFish = makeMakeFish(() => ({
      where: seedEvent,
      isReset: () => {
        throw new Error('broken')
      },
    }))

    initFish(pond, 4, 4, 4)

    await expect(readState(pond, makeFish)).rejects.toMatchObject({ message: 'broken' })
    expect(reported).toMatchObject({ entityType: 'hello', name: '4' })

    pond.dispose()
  })
})
