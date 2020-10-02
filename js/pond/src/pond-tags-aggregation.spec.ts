import { Observable } from 'rxjs'
import { Fish, FishId, noEvents, Pond, Tag, Tags, Where } from '.'

const emitTestEvents = async (pond: Pond) => {
  await pond.emit(Tags('t0', 't1', 't2'), 'hello').toPromise()
  await pond.emit(Tags('t0', 't1', 't2'), 'world').toPromise()
  await pond.emit(Tag('t1'), 't1 only').toPromise()
  await pond.emit(Tag('t2'), 't2 only').toPromise()
}

const assertStateAndDispose = async <S>(states: Observable<S>, expected: S, pond: Pond) => {
  const res = states
    .debounceTime(5)
    .take(1)
    .toPromise()

  await expect(res).resolves.toEqual(expected)

  pond.dispose()
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const aggregateAsObservable = <S>(pond: Pond, agg: Fish<S, any>): Observable<S> =>
  new Observable(x => {
    pond.observe(agg, s => x.next(s))
  })

describe('application of commands in the pond', () => {
  const expectAggregationToYield = async (
    subscriptions: Where<string>,
    expectedResult: string[],
  ) => {
    const pond = Pond.test()

    const aggregate = Fish.eventsDescending<string>(subscriptions)

    const res = aggregateAsObservable(pond, aggregate)

    await emitTestEvents(pond)

    await assertStateAndDispose(res, expectedResult, pond)
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

  it('should aggregate nothing', async () =>
    // Empty union means not a single subscription
    expectAggregationToYield(noEvents, []))

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

      const unsubscribe0 = pond.observe<State, Event>(aggregate0, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use same name, but different subscriptions, to assert that cached aggregate is re-used
      const aggregate1 = mkAggregate(Tag('t99'))

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertStateAndDispose(res, ['t1 only', 'world', 'hello'], pond)
    })

    it('should cache based on key, across unsubscribe calls', async () => {
      const pond = Pond.test()

      const aggregate = mkAggregate(Tag('t1'))

      const unsubscribe0 = pond.observe<State, Event>(aggregate, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)
      unsubscribe0()
      await Observable.timer(500).toPromise()

      // should immediately pick up the existing aggregation's pipeline
      const res = new Promise((resolve, _reject) =>
        pond.observe(aggregate, state => resolve(state)),
      )

      await expect(res).resolves.toEqual(['t1 only', 'world', 'hello'])

      pond.dispose()
    })

    it('should permit different aggregations in parallel', async () => {
      const pond = Pond.test()

      const aggregate0 = mkAggregate(Tag('t0'))

      const unsubscribe0 = pond.observe<State, Event>(aggregate0, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use a different cache key to start another aggregation
      const aggregate1 = mkAggregate(Tag('t1'), FishId.of('x', 'different-name', 0))

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertStateAndDispose(res, ['t1 only', 'world', 'hello'], pond)
    })
  })
})
