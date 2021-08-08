/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { Observable, Scheduler } from '../../node_modules/rxjs'
import { marbles } from 'rxjs-marbles'
import { takeWhileInclusive } from './observable'

describe('takeWhileInclusive', () => {
  it('should takeWhile predicate and then emit one more', async () => {
    const result = await Observable.from([1, 2, 3, 4, 5, 6, 6, 7])
      .pipe(takeWhileInclusive(e => e < 6))
      .toArray()
      .toPromise()
    expect(result).toEqual([1, 2, 3, 4, 5, 6]) // just one 6, the one on which the predicate has fired
    const result2 = await Observable.from([1, 2, 3, 4, 5, 6, 6, 7, 8, 8, 9])
      .pipe(takeWhileInclusive(e => e < 8))
      .toArray()
      .toPromise()
    expect(result2).toEqual([1, 2, 3, 4, 5, 6, 6, 7, 8]) // just one 8
  })

  it('should unsubscribe when done with stuff', async () => {
    let count = 0
    const result = await Observable.from([1, 2, 3, 4, 5, 6, 7, 8, 9])
      .observeOn(Scheduler.queue)
      .do(_ => ++count)
      .takeWhile(e => e < 6)
      .toArray()
      .toPromise()

    let count2 = 0
    const result2 = await Observable.from([1, 2, 3, 4, 5, 6, 7, 8, 9])
      .observeOn(Scheduler.queue)
      .do(_ => ++count2)
      .pipe(takeWhileInclusive(e => e < 5))
      .toArray()
      .toPromise()
    expect(result).toEqual([1, 2, 3, 4, 5])
    expect(count).toEqual(6)
    expect(result2).toEqual([1, 2, 3, 4, 5])
    expect(count2).toEqual(5)
  })

  it(
    'should unsubscribe from upstream after it gets the result 2',
    marbles(m => {
      const values = { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7 }

      const a = m.hot('^-a-b----c-d-e-f-g-|', values)
      const bsubs = '^---!'
      const e = m.hot('^-a-(b|)', values)

      const c = a.pipe(takeWhileInclusive(z => z < 2))
      m.expect(c).toBeObservable(e)
      m.expect(a).toHaveSubscriptions(bsubs)
    }),
  )

  it('should handle errors', async () => {
    let count = 0
    const result2 = await Observable.from([1, 2, 3, 4])
      .concat(Observable.throw('BOOM!'))
      .concat(Observable.from([6, 7, 8, 9]))
      .do(_ => ++count)
      .pipe(takeWhileInclusive(e => e < 10))
      .catch(e => Observable.of(e))
      .toArray()
      .toPromise()
    expect(result2).toEqual([1, 2, 3, 4, 'BOOM!'])
    expect(count).toEqual(4)
  })
})
