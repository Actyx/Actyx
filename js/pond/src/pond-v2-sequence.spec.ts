/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { Fish, FishId, Pond, Reduce, StateEffect, Tag } from '.'

export type State = { n: number; fill: number }

type CompareAndIncrement = { type: 'set'; n: number }
type Fill = { type: 'fill' }
export type Payload = CompareAndIncrement | Fill

const onEvent: Reduce<State, Payload> = (state: State, event: Payload) => {
  if (event.type === 'fill') {
    state.fill += 1
    return state
  }

  if (state.n !== event.n - 1) {
    throw new Error(`expected state to be ${event.n - 1}, but was ${state.n}`)
  }

  state.n = event.n
  return state
}

const agg: Fish<State, Payload> = {
  where: Tag('self'),

  initialState: { n: 0, fill: 0 },

  onEvent,

  fishId: FishId.of('x', 'x', 0),
}

const setN: (n: number) => StateEffect<State, Payload> = (n) => (state, enQ) => {
  if (state.n !== n - 1) {
    throw new Error(`expected state to be ${n - 1}, but was ${state.n}`)
  }
  const payload: CompareAndIncrement = { type: 'set', n }

  return enQ(Tag<Payload>('self'), payload)
}

const checkN: (expected: number) => StateEffect<State, never> = (expected) => (state, _enQ) => {
  if (state.n !== expected) {
    throw new Error(`expected state to be ${expected}, but was ${state.n}`)
  }
}

describe('application of commands in the pond v2', () => {
  const expectState = (pond: Pond, expected: number, aggr = agg): Promise<State> =>
    new Promise((resolve, _reject) =>
      pond.observe(aggr, (state) => state.n === expected && resolve(state)),
    )

  describe('raw state effects', () => {
    it('should run state effect, regardless of user awaiting the promise', async () => {
      const pond = Pond.test()

      const run = (x: StateEffect<State, Payload>) => pond.run(agg, x)

      // Assert it’s run even if we don’t subscribe
      run(setN(1))

      await expectState(pond, 1)

      await run(setN(2)).toPromise()
      await run(setN(3)).toPromise()

      await expectState(pond, 3)

      pond.dispose()
    })

    it('should propagate errors if the user subscribes', async () => {
      const pond = Pond.test()

      await expect(pond.run(agg, setN(2)).toPromise()).rejects.toEqual(
        new Error('expected state to be 1, but was 0'),
      )

      await expect(pond.run(agg, checkN(20)).toPromise()).rejects.toEqual(
        new Error('expected state to be 20, but was 0'),
      )

      pond.dispose()
    })

    it('effects should wait for application of previous', async () => {
      const pond = Pond.test()

      const r = (x: StateEffect<State, Payload>) => pond.run(agg, x)
      for (let i = 1; i <= 1000; i++) {
        r(setN(i))
      }

      // We can tell there weren’t any errors from us having gone up all the way to 1000.
      await expectState(pond, 1000)

      pond.dispose()
    })

    it('should be cached correctly, even if observe goes first', async () => {
      const pond = Pond.test()

      const cancel = pond.observe(agg, (_) => {
        /* just drop it */
      })

      const run = (x: StateEffect<State, Payload>) => pond.run(agg, x)

      await run(setN(1)).toPromise()

      await expectState(pond, 1)

      await run(setN(2)).toPromise()
      await run(setN(3)).toPromise()

      await expectState(pond, 3)

      cancel()
      pond.dispose()
    })
  })

  describe('automatic effects', () => {
    const autoBump: StateEffect<State, CompareAndIncrement> = (state, enQ) =>
      enQ(Tag<CompareAndIncrement>('self'), { type: 'set', n: state.n + 1 })

    it('should run until cancellation condition', async () => {
      const pond = Pond.test()

      pond.keepRunning(agg, autoBump, (state: State) => state.n === 100)

      await expectState(pond, 100)

      // Make sure the effect has stopped by manually bumping the state ourselves.
      await pond.run(agg, checkN(100)).toPromise()

      await pond.run(agg, setN(101)).toPromise()
      await expectState(pond, 101)
      await pond.run(agg, checkN(101)).toPromise()

      pond.dispose()
    })

    it('should respect sequence also when effect async', async () => {
      const pond = Pond.test()

      const delayedBump: StateEffect<State, Payload> = (state, enQ) =>
        new Promise((resolve, _reject) =>
          setTimeout(() => resolve(enQ(Tag<Payload>('self'), { type: 'set', n: state.n + 1 })), 5),
        )

      const stateIs10 = expectState(pond, 10)

      const c = pond.keepRunning(agg, delayedBump)

      await stateIs10

      c()
      pond.dispose()
    })

    it("should wait for the actual effect's events to be processed, ignore other events that may come in", async () => {
      const pond = Pond.test()

      pond.keepRunning(agg, autoBump, (state: State) => state.n === 40)

      const emitFill = () => pond.emit(Tag('self'), { type: 'fill' })

      const timer = setInterval(emitFill, 3)

      const s = await expectState(pond, 40)
      // Just make sure some fill events were in fact processed.
      expect(s.fill).toBeGreaterThan(0)

      clearInterval(timer)

      pond.dispose()
    })

    it('should run parallel to user effects', async () => {
      const pond = Pond.test()

      pond.keepRunning<State, Payload>(
        agg,
        // We skip increasing 5, depend on our manual calls to do it.
        (state, enQ) => {
          if (state.n !== 5) {
            enQ(Tag<Payload>('self'), { type: 'set', n: state.n + 1 })
          }
        },
        (state: State) => state.n === 10,
      )

      const effect = setN(6)
      let success = false

      const tryBumpTo6 = () => {
        if (success) {
          return
        }

        pond
          .run(agg, effect)
          .toPromise()
          .then(
            () => (success = true),
            () => {
              /* some rejections are expected */
            },
          )
      }

      const q = setInterval(tryBumpTo6, 1)

      await expectState(pond, 10)

      clearInterval(q)

      pond.dispose()
    })

    // Bump only even numbers
    const bumpEven: StateEffect<State, CompareAndIncrement> = (state, enQ) => {
      if (state.n % 2 === 0) {
        enQ<CompareAndIncrement>(Tag('self'), { type: 'set', n: state.n + 1 })
      }
    }

    it('should run parallel to user effects 2', async () => {
      const pond = Pond.test()

      pond.keepRunning<State, Payload>(agg, bumpEven)

      await expectState(pond, 1)

      pond.run(agg, setN(2))

      // Bumped up to 3 already
      await expectState(pond, 3)

      pond.dispose()
    })

    it('should run multiple auto effects in parallel', async () => {
      const pond = Pond.test()
      const tags = Tag<Payload>('self')

      const stateIs15 = expectState(pond, 15)

      const mk =
        (remainder: number): StateEffect<State, Payload> =>
        (state, enQ) => {
          if (state.n % 3 === remainder) {
            enQ(tags, { type: 'set', n: state.n + 1 })
          }
        }

      pond.keepRunning(agg, mk(0), (s) => s.n === 20)
      pond.keepRunning(agg, mk(1), (s) => s.n === 20)
      pond.keepRunning(agg, mk(2), (s) => s.n === 20)

      await stateIs15
      await expectState(pond, 20)

      pond.dispose()
    })

    it('should run multiple auto effects in parallel, even if they all always fire', async () => {
      const pond = Pond.test()
      const tags = Tag<Payload>('self')

      const mk =
        (remainder: number): StateEffect<State, Payload> =>
        (state, enQ) => {
          if (state.n % 3 === remainder) {
            enQ(tags, { type: 'set', n: state.n + 1 })
          } else {
            enQ(tags, { type: 'fill' })
            enQ(tags, { type: 'fill' })
          }
        }

      pond.keepRunning(agg, mk(0), (s) => s.n === 10)
      pond.keepRunning(agg, mk(1), (s) => s.n === 10)
      pond.keepRunning(agg, mk(2), (s) => s.n === 10)

      await expectState(pond, 10)

      pond.dispose()
    })

    it('should be cancellable', async () => {
      const pond = Pond.test()

      const cancel = pond.keepRunning(agg, bumpEven)

      await expectState(pond, 1)
      cancel()

      pond.run(agg, setN(2))

      await expectState(pond, 2)

      pond.dispose()
    })

    // FIXME: This is too flaky on CI
    it.skip('should be cancellable pretty swiftly', async () => {
      const pond = Pond.test()

      const cancel = pond.keepRunning(agg, autoBump)

      // This is only really reliable as long as we debounce the automatic effect.
      pond.observe(agg, (state) => state.n > 1000 && cancel())

      await expectState(pond, 1001)

      pond.dispose()
    })
  })

  describe('automatic effects with event sent to other aggregates', () => {
    const mkAgg = (name: string) => ({
      where: Tag<Payload>(name),

      initialState: { n: 0, fill: 0 },

      onEvent,

      fishId: FishId.of('seqtest', name, 0),
    })

    const alpha = mkAgg('alpha')
    const beta = mkAgg('beta')

    it('should be able to pingpong', async () => {
      const pond = Pond.test()

      const stateIs30 = expectState(pond, 30, beta)

      const c0 = pond.keepRunning(alpha, (state, enQ) =>
        enQ(Tag('beta'), { type: 'set', n: state.n + 1 }),
      )

      await expectState(pond, 1, beta)

      const c1 = pond.keepRunning(beta, (state, enQ) =>
        enQ(Tag('beta'), { type: 'set', n: state.n + 1 }),
      )

      await stateIs30

      c0()
      c1()
      pond.dispose()
    })
  })
})
