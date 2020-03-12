/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { RunStats } from './runStats'

describe('runStats', () => {
  it('should init and calculate the current values', () => {
    const runStats = RunStats.create()
    expect(runStats.counters.current()).toEqual({})
    expect(runStats.durations.getAndClear()).toEqual({})
    expect(runStats.gauges.current()).toEqual({})

    runStats.counters.add('a') // create a counter (add has a default argument of one)
    expect(runStats.counters.current()).toEqual({ a: [1, 1] })

    runStats.counters.add('a', 2)
    expect(runStats.counters.current()).toEqual({ a: [3, 2] })

    runStats.durations.add('a', 0)
    expect(runStats.durations.getAndClear()).toEqual({
      a: {
        count: 1,
        min: 0,
        median: 0,
        _90: 0,
        _95: 0,
        _99: 0,
        max: 0,
        pending: 0,
        discarded: 0,
      },
    })
    runStats.durations.add('a', 0)
    runStats.durations.add('a', 1)
    expect(runStats.durations.getAndClear()).toEqual({
      a: {
        count: 2,
        min: 0,
        median: 1,
        _90: 1,
        _95: 1,
        _99: 1,
        max: 1,
        pending: 0,
        discarded: 0,
      },
    })

    runStats.gauges.set('foo', 7729) // set a gauge
    const gauges = runStats.gauges.current()
    expect(gauges).toEqual({ foo: { last: 7729, max: 7729 } })

    runStats.gauges.set('foo', 4711) // set the same gauge
    expect(gauges).toEqual({ foo: { last: 7729, max: 7729 } }) // don't change the read only copy
    expect(runStats.gauges.current()).toEqual({ foo: { last: 4711, max: 7729 } })

    runStats.gauges.set('foo', 9999)
    expect(runStats.gauges.current()).toEqual({ foo: { last: 9999, max: 9999 } })
  })

  it('should init and calculate for observable pipeline', async () => {
    const runStats = RunStats.create()
    const profileObservable = runStats.profile.profileObservable('test', Number.MAX_SAFE_INTEGER)

    const foo = Observable.from([1, 2, 3])
      .delay(1000)
      .pipe(profileObservable)
      .toPromise()

    await foo
    expect(runStats.counters.current()).toEqual({})
    const test = runStats.durations.getAndClear().test

    if (!('count' in test)) {
      fail('should have had stats')
      return
    }
    expect(test.max).toBeGreaterThan(800000)
    // 4 because there is one pending operation
    expect(test.count).toEqual(4)
  })

  it('should profile sync block', () => {
    const runStats = RunStats.create()
    const res = runStats.profile.profileSync('test')(() => {
      let result = 0

      for (let i = 1; i <= 1000000; i++) {
        result += i
      }
      return result
    })
    const test = runStats.durations.getAndClear().test
    if (!('count' in test)) {
      fail('should have had stats')
      return
    }
    expect(test.count).toEqual(1)
    expect(res).toEqual(500000500000)
  })

  it('should log discarded entries', () => {
    const runStats = RunStats.create()
    runStats.durations.end('unknown', 0, 12)
    runStats.durations.start('known', 10)
    runStats.durations.end('known', 10, 25)
    runStats.durations.end('known', 1, 2)
    expect(runStats.durations.getAndClear()).toEqual({
      unknown: {
        pending: 0,
        discarded: 1,
      },
      known: {
        count: 1,
        pending: 0,
        discarded: 1,
        min: 15,
        median: 15,
        _90: 15,
        _95: 15,
        _99: 15,
        max: 15,
      },
    })
  })

  it('should produce times for ongoing operations', async () => {
    const runStats = RunStats.create()
    const profileObservable = runStats.profile.profileObservable('test')
    // very long-running profiled operation
    const foo = Observable.timer(100000).pipe(profileObservable)

    for (let i = 0; i < 10; i++) {
      foo.subscribe()
    }
    // wait a second
    await Observable.timer(1000).toPromise()
    const stats = runStats.durations.getAndClear()
    // all of the long-running operations should still be pending
    const { pending } = stats.test
    expect('count' in stats.test).toBeFalsy()
    expect(pending).toEqual(10)
  })

  it('should be roughly correct', () => {
    const runStats = RunStats.create()

    for (let i = 0; i <= 10; i++) {
      runStats.durations.add('test', i * 1000)
    }
    const stats = runStats.durations.getAndClear().test
    if (!('count' in stats)) {
      fail('got no values in stats')
      return
    }
    const { count, min, median, _90, _95, _99, max, pending } = stats

    expect(count).toEqual(11)
    expect(min).toEqual(0)
    expect(median).toEqual(5000)
    expect(_90).toEqual(9000)
    expect(_95).toEqual(10000)
    expect(_99).toEqual(10000)
    expect(max).toEqual(10000)
    expect(pending).toEqual(0)

    expect(runStats.durations.getAndClear().test).toBeUndefined()
  })
})
