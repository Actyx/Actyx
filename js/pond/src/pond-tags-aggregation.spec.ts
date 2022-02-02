/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { Observable, lastValueFrom, Subject, timer } from '../node_modules/rxjs'
import { debounceTime, take } from '../node_modules/rxjs/operators'
import { Fish, FishErrorContext, FishErrorReporter, FishId, Pond, Tag, Tags, Where } from '.'

const emitTestEvents = async (pond: Pond) => {
  await pond.emit(Tags('t0', 't1', 't2'), 'hello').toPromise()
  await pond.emit(Tags('t0', 't1', 't2'), 'world').toPromise()
  await pond.emit(Tag('t1'), 't1 only').toPromise()
  await pond.emit(Tag('t2'), 't2 only').toPromise()
}

const assertEventualState = async <S>(states: Observable<S>, expected: S) => {
  const res = lastValueFrom(states.pipe(debounceTime(5), take(1)))

  await expect(res).resolves.toEqual(expected)
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const aggregateAsObservable = <S>(pond: Pond, agg: Fish<S, any>): Observable<S> =>
  new Observable((x) => {
    pond.observe(agg, (s) => x.next(s))
  })

describe('tag-based aggregation (Fish observe) in the Pond', () => {
  const expectAggregationToYield = async (
    subscriptions: Where<string>,
    expectedResult: string[],
  ) => {
    const pond = Pond.test()

    const aggregate = Fish.eventsDescending<string>(subscriptions)

    const res = aggregateAsObservable(pond, aggregate)

    await emitTestEvents(pond)

    await assertEventualState(res, expectedResult)

    // Assert that Pond.currentState gives the same result
    const readAsPromise = await pond.currentState(aggregate)
    expect(readAsPromise).toEqual(expectedResult)

    pond.dispose()
  }

  it('should aggregate based on tags intersection', async () =>
    expectAggregationToYield(Tags('t0', 't1'), ['world', 'hello']))

  it('should aggregate based on tags union', async () =>
    expectAggregationToYield(Tag<string>('t0').or(Tag<string>('t1')), [
      't1 only',
      'world',
      'hello',
    ]))

  it('should aggregate based on single tag', async () =>
    expectAggregationToYield(Tag('t2'), ['t2 only', 'world', 'hello']))

  it('should aggregate everything', async () =>
    // Empty intersection matches everything
    expectAggregationToYield(Tags(), ['t2 only', 't1 only', 'world', 'hello']))

  describe('error handling', () => {
    const brokenFish: Fish<string, string> = {
      onEvent: (_state, event) => {
        if (event === 'error') {
          throw new Error('oh, I am broken')
        }

        return event
      },
      initialState: 'initial',
      where: Tag<string>('t1'),
      fishId: FishId.of('broken', 'test', 1),
    }

    const setup = (
      errorCb?: (err: unknown) => void,
      fishExt: Partial<Fish<string, string>> = {},
    ) => {
      const reported: Subject<FishErrorContext> = new Subject()
      const fishErrorReporter: FishErrorReporter = (_err, _fishId, detail) => {
        reported.next(detail)
      }

      const pond = Pond.test({ fishErrorReporter })

      const fish = {
        ...brokenFish,
        ...fishExt,
      }

      let latestState: string = 'unset'
      pond.observe(
        fish,
        (s) => {
          latestState = s
        },
        errorCb,
      )

      const emitEventSequenceWithError = async () => {
        await pond.emit(Tag('t1'), 't1 event 1').toPromise()
        await pond.emit(Tag('t1'), 'error').toPromise()
        await pond.emit(Tag('t1'), 't1 event 2').toPromise()
      }

      const rejectCurrentState = () =>
        expect(pond.currentState(brokenFish)).rejects.toMatchObject({ message: 'oh, I am broken' })

      return {
        pond,
        emitEventSequenceWithError,
        errors: reported,
        assertLatestState: (expected: string) => expect(latestState).toEqual(expected),
        rejectCurrentState,
      }
    }

    it('should pass at least the last good state to the callback, even if an error has been thrown', async () => {
      const { pond, emitEventSequenceWithError, assertLatestState, errors, rejectCurrentState } =
        setup()

      const nextErr = lastValueFrom(errors.pipe(take(1)))
      await emitEventSequenceWithError()

      await expect(nextErr).resolves.toMatchObject({ occuredIn: 'onEvent' })
      await rejectCurrentState()

      assertLatestState('t1 event 1')

      let stateCb2: ((s: string) => void) | undefined
      const latestState2 = new Promise<string>((resolve) => {
        stateCb2 = resolve
      })
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      pond.observe(brokenFish, stateCb2!)
      await expect(latestState2).resolves.toEqual('t1 event 1')

      pond.dispose()
    })

    it('should propagate errors to the supplied error callback', async () => {
      let stoppedByError
      const reportedErr = new Promise((resolve) => {
        stoppedByError = resolve
      })

      const { pond, emitEventSequenceWithError, assertLatestState, rejectCurrentState } =
        setup(stoppedByError)

      assertLatestState('unset')
      await emitEventSequenceWithError()

      await expect(reportedErr).resolves.toBeDefined()
      await rejectCurrentState()
      assertLatestState('t1 event 1')

      pond.dispose()
    })

    it('should supply final value, then call stoppedByError', async () => {
      let reportedErr = null
      const stoppedByError = (err: unknown) => {
        reportedErr = err
      }

      const { pond, emitEventSequenceWithError, assertLatestState, rejectCurrentState } =
        setup(stoppedByError)

      assertLatestState('unset')
      await emitEventSequenceWithError()

      expect(reportedErr).toBeDefined()

      let reportErrCb2
      const reportedErr2 = new Promise((resolve) => {
        reportErrCb2 = resolve
      })
      let latestState2: string = 'unset'
      pond.observe(
        brokenFish,
        (s) => {
          latestState2 = s
        },
        reportErrCb2,
      )
      await expect(reportedErr2).resolves.toBeDefined()
      expect(latestState2).toEqual('t1 event 1')

      await rejectCurrentState()

      pond.dispose()
    })

    it('should report if error was caused by isReset', async () => {
      const { pond, emitEventSequenceWithError, assertLatestState, errors, rejectCurrentState } =
        setup(undefined, {
          isReset: (ev) => {
            if (ev === 'error') {
              throw new Error('oh, I am broken')
            }
            return true
          },
        })

      const nextErr = lastValueFrom(errors.pipe(take(1)))
      assertLatestState('unset')
      await emitEventSequenceWithError()

      await expect(nextErr).resolves.toMatchObject({ occuredIn: 'isReset' })
      await rejectCurrentState()
      assertLatestState('t1 event 1')

      pond.dispose()
    })

    it('should report if error was caused by deserializeState', async () => {
      const { pond, emitEventSequenceWithError, assertLatestState, errors, rejectCurrentState } =
        setup(undefined, {
          onEvent: (x) => x,
          deserializeState: () => {
            throw new Error('oh, I am broken')
          },
        })

      const nextErr = lastValueFrom(errors.pipe(take(1)))
      assertLatestState('unset')
      await emitEventSequenceWithError()

      await expect(nextErr).resolves.toMatchObject({ occuredIn: 'deserializeState' })
      await rejectCurrentState()
      assertLatestState('unset')

      pond.dispose()
    })
  })

  describe('caching', () => {
    type Event = string
    type State = ReadonlyArray<Event>

    const mkAggregate = (subscriptions: Where<Event>, fishId = FishId.of('x', 'x', 0)) => ({
      where: subscriptions,

      initialState: [],

      onEvent: (state: State, event: Event) => [event, ...state],

      fishId,
    })

    it('should cache based on key', async () => {
      const pond = Pond.test()

      const aggregate0 = mkAggregate(Tag('t1'))

      const unsubscribe0 = pond.observe<State, Event>(aggregate0, (_s) => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use same name, but different subscriptions, to assert that cached aggregate is re-used
      const aggregate1 = mkAggregate(Tag('t99'))

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertEventualState(res, ['t1 only', 'world', 'hello'])
      pond.dispose()
    })

    it('should cache based on key, but always invoke callback with delay, so that cancelation works', async () => {
      const pond = Pond.test()

      const aggregate0 = mkAggregate(Tag('t1'))

      await emitTestEvents(pond)
      const res = await lastValueFrom(aggregateAsObservable(pond, aggregate0).pipe(take(1)))
      expect(res).toEqual(['t1 only', 'world', 'hello'])

      const invocation = new Promise<boolean>((resolve, reject) => {
        const unsubscribe1: () => void = pond.observe<State, Event>(aggregate0, (_s) => {
          try {
            // Assert unsubscribe1 is defined already and invoking it does not throw an exception
            unsubscribe1()
            resolve(true)
          } catch (e) {
            reject(e)
          }
        })
      })

      await expect(invocation).resolves.toBeTruthy()

      pond.dispose()
    })

    it('should cache based on key, across unsubscribe calls', async () => {
      const pond = Pond.test()

      const aggregate = mkAggregate(Tag('t1'))

      const unsubscribe0 = pond.observe<State, Event>(aggregate, (_s) => {
        /* swallow */
      })

      await emitTestEvents(pond)
      unsubscribe0()
      await lastValueFrom(timer(500))

      // should immediately pick up the existing aggregation's pipeline
      const res = new Promise((resolve, _reject) =>
        pond.observe(aggregate, (state) => resolve(state)),
      )

      await expect(res).resolves.toEqual(['t1 only', 'world', 'hello'])

      pond.dispose()
    })

    it('should permit different aggregations in parallel', async () => {
      const pond = Pond.test()

      const aggregate0 = mkAggregate(Tag('t0'))

      const unsubscribe0 = pond.observe<State, Event>(aggregate0, (_s) => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use a different cache key to start another aggregation
      const aggregate1 = mkAggregate(Tag('t1'), FishId.of('x', 'different-name', 0))

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertEventualState(res, ['t1 only', 'world', 'hello'])
      pond.dispose()
    })
  })
})

describe('Fish declarations Tag checking', () => {
  type T0 = {
    type: '0'
    t0: object
  }

  type A = {
    type: 'A'
    data0: number
  }

  type B = {
    type: 'B'
    data1: string
  }

  type C = {
    type: 'C'
    data1: number
  }

  const tagA = Tag<A>('A')

  // Tag that covers 3 types
  const abcTag = Tag<A | B | C>('ABC')

  // Satisfy TS (no unused var)
  const ignoreUnusedVar = (_v: unknown) => undefined

  const fishArgs = {
    onEvent: (state: undefined, _payload: A | B) => state,
    initialState: undefined,
    fishId: FishId.of('f', 'a', 0),
  }

  it('should require fish to implement onEvent that can handle all incoming events', () => {
    const fishWrong: Fish<undefined, A | B> = {
      ...fishArgs,

      // @ts-expect-error for too large subscription set
      where: abcTag,
    }

    ignoreUnusedVar(fishWrong)
  })

  it('fish should accept direct Where<unknown> indicating untyped tag query', () => {
    const fishRight1: Fish<undefined, A | B> = {
      ...fishArgs,

      // Automatically type-inferred to match Fish declaration
      where: Tags('some', 'plain', 'tags'),
    }

    const fishRight2: Fish<undefined, A | B> = {
      ...fishArgs,

      // Automatically type-inferred to match Fish declaration
      where: Tag('a-single-plain-tag'),
    }

    ignoreUnusedVar(fishRight1)
    ignoreUnusedVar(fishRight2)
  })

  it('should accept OR-concatentation of untyped queries with explicit cast', () => {
    const fishWrong: Fish<undefined, A | B> = {
      ...fishArgs,

      // @ts-expect-error since without cast, this will fail
      where: Tags('1', '2').or(Tag('foo')),
    }

    ignoreUnusedVar(fishWrong)

    const fishRight: Fish<undefined, A | B> = {
      ...fishArgs,

      // ... but adding an explicit cast solves the problem
      where: Tags('1', '2').or(Tag('foo')) as Where<A | B>,
    }

    ignoreUnusedVar(fishRight)
  })

  it('should accept additional untyped tags on an intersection', () => {
    const fishRight: Fish<undefined, A | B> = {
      ...fishArgs,

      // Type remains Tags<A>
      where: tagA.and('some-other-tag'),
    }

    ignoreUnusedVar(fishRight)
  })

  it('should accept additional tags on an intersection', () => {
    const fishRight: Fish<undefined, A | B> = {
      ...fishArgs,

      // Type remains Tags<A>
      where: tagA.withId('n').and(Tag('some-other-tag').withId('foo')),
    }

    ignoreUnusedVar(fishRight)
  })

  it('should accept additional tags with explicit cast', () => {
    const fishWrong: Fish<undefined, A | B> = {
      ...fishArgs,

      // @ts-expect-error since without cast, this will fail
      where: Tag('q').withId('n').and(Tag('some-other-tag').withId('foo')),
    }

    const fishRight: Fish<undefined, A | B> = {
      ...fishArgs,

      // Casting works
      where: Tag('q').withId('n').and(Tag('some-other-tag').withId('foo')) as Where<A>,
    }

    ignoreUnusedVar(fishWrong)
    ignoreUnusedVar(fishRight)
  })

  it('should allow fish to handle more events than indicated by tags', () => {
    const fishRight: Fish<undefined, A | B | C | T0> = {
      onEvent: (state: undefined, _payload: A | B | C | T0) => state,
      initialState: undefined,
      fishId: FishId.of('f', 'a', 0),

      // Fish declares it handles T0 also -> no problem.
      where: abcTag,
    }

    ignoreUnusedVar(fishRight)
  })
})
