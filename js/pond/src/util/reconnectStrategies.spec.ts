/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable } from 'rxjs'
import { marbles } from 'rxjs-marbles'
import reconnectStrategies from './reconnectStrategies'

// see https://github.com/redux-observable/redux-observable/issues/180
const injectTimeBasedOperators = (testScheduler: any) => {
  const originalDelay = Observable.prototype.delay
  function stubDelay(this: any, dueTime: number) {
    return originalDelay.call(this, dueTime, testScheduler)
  }
  spyOn(Observable.prototype, 'delay').and.callFake(stubDelay)
}

describe('retry', () => {
  it(
    'should give up after the given number of retries',
    marbles(m => {
      injectTimeBasedOperators(m.scheduler)
      const i = 'x--x--x--x--x--x'
      const o = '-x--x--x-#'
      const errors = m.cold(i)
      const result = reconnectStrategies.retry({ attempts: 3, delayMs: 10 })(errors)
      return m.expect(result).toBeObservable(o, { x: 'x' }, new Error('Giving up after 3 retries!'))
    }),
  )

  it(
    'should use the given delay',
    marbles(m => {
      injectTimeBasedOperators(m.scheduler)
      const i = 'x--x'
      const o = '---x--x'
      const errors = m.cold(i)
      const result = reconnectStrategies.retry({ delayMs: 30 })(errors)
      return m.expect(result).toBeObservable(o)
    }),
  )
})

describe('exponentialBackoff', () => {
  it(
    'should give up after the given number of retries',
    marbles(m => {
      injectTimeBasedOperators(m.scheduler)
      const i = 'x------x------x-------x'
      const o = '-x-------x--------x---#'
      const errors = m.cold(i)
      const result = reconnectStrategies.exponentialBackoff({ attempts: 3, minDelay: 10 })(errors)
      return m.expect(result).toBeObservable(o, { x: 'x' }, new Error('Giving up after 3 retries!'))
    }),
  )
})
