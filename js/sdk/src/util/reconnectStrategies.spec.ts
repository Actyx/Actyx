/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import reconnectStrategies from './reconnectStrategies'
import { TestScheduler } from 'rxjs/testing'

const scheduler = () =>
  new TestScheduler((actual, expected) => {
    expect(actual).toEqual(expected)
  })

describe('retry', () => {
  it('should give up after the given number of retries', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('x--x--x-x')
      const expected = '   -x--x--x#'
      const result = reconnectStrategies.retry({
        attempts: 3,
        delayMs: 1,
      })(errors)

      expectObservable(result).toBe(expected, undefined, new Error('Giving up after 3 retries!'))
    }))
  // This is the implementation. Note sure it actually makes sense.
  it('should work use default 1000ms when given 0', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('abc')
      const expected = '1000ms abc'
      const result = reconnectStrategies.retry({
        delayMs: 0,
      })(errors)

      expectObservable(result).toBe(expected)
    }))
  it('should use the given delay (1)', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('a-b')
      const expected = '   -a-b'
      const result = reconnectStrategies.retry({
        delayMs: 1,
      })(errors)

      expectObservable(result).toBe(expected)
    }))

  it('should use the given delay (2)', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('ab')
      const expected = '   --------ab'
      const result = reconnectStrategies.retry({
        delayMs: 8,
      })(errors)

      expectObservable(result).toBe(expected)
    }))

  it('should use the given delay (3)', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('a---b')
      const expected = '   90ms a --- b'
      const result = reconnectStrategies.retry({
        delayMs: 90,
      })(errors)

      expectObservable(result).toBe(expected)
    }))
})

describe('exponential backoff', () => {
  it('should exponentially back off', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('a-b-c-d')
      const expected = '  1000ms a - 1000ms b - 2000ms c - 4000ms d'
      const result = reconnectStrategies.exponentialBackoff({
        minDelay: 1000,
      })(errors)

      expectObservable(result).toBe(expected)
    }))

  it('should give up after the given number of retries', () =>
    scheduler().run(({ cold, expectObservable }) => {
      const errors = cold('abcd')
      const expected = '   ---#'
      const result = reconnectStrategies.exponentialBackoff({
        minDelay: 1000,
        attempts: 3,
      })(errors)

      expectObservable(result).toBe(expected, undefined, new Error('Giving up after 3 retries!'))
    }))
})
