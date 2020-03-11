/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { TestScheduler } from 'rxjs'
import { Observable } from 'rxjs/Observable'
import { Milliseconds } from '../types'
import { backoffDelay, BackoffStrategy, retryOnErrorWithBackoff } from './backoffDelay'

describe('retryOnErrorWithBackoff()', () => {
  it('should pass', () => {
    const scheduler = new TestScheduler((r, expected) => {
      expect(r).toEqual(expected)
    })
    const source = scheduler.createColdObservable('aa(a|')
    const result = retryOnErrorWithBackoff(source, undefined, undefined, scheduler)
    scheduler.expectObservable(result).toBe('aa(a|')
    scheduler.flush()
  })

  it('should fail', done => {
    const source = Observable.of(1, 3, 5, 7, 9).map(() => {
      throw new Error('fail')
    })

    retryOnErrorWithBackoff(source, BackoffStrategy.linear(1), {
      maxRetries: 1,
    }).subscribe({
      error: (x: any) => {
        expect(x.message).toEqual('fail')
        done()
      },
    })
  })
})

describe('backoffDelay()', () => {
  it('exponential', () => {
    const scheduler = new TestScheduler(r => {
      expect(r.map(({ frame, notification: { kind } }: any) => ({ frame, kind }))).toEqual([
        { frame: 1, kind: 'N' },
        { frame: 12, kind: 'N' },
        { frame: 24, kind: 'N' },
        { frame: 38, kind: 'N' },
        { frame: 56, kind: 'N' },
        { frame: 82, kind: 'N' },
        { frame: 124, kind: 'N' },
        { frame: 198, kind: 'N' },
        { frame: 336, kind: 'N' },
        { frame: 602, kind: 'N' },
        { frame: 602, kind: 'C' },
      ])
    })
    const source = scheduler.createColdObservable(`aaaaaaaaa(a|`)
    const result = backoffDelay(BackoffStrategy.exponential(1), { maxRetries: 100 }, scheduler)(
      source,
    )
    scheduler.expectObservable(result).toBe('')
    scheduler.flush()
  })

  it('linear', () => {
    const scheduler = new TestScheduler(r => {
      expect(r.map(({ frame, notification: { kind } }: any) => ({ frame, kind }))).toEqual([
        { frame: 1, kind: 'N' },
        { frame: 12, kind: 'N' },
        { frame: 23, kind: 'N' },
        { frame: 34, kind: 'N' },
        { frame: 45, kind: 'N' },
        { frame: 56, kind: 'N' },
        { frame: 67, kind: 'N' },
        { frame: 78, kind: 'N' },
        { frame: 89, kind: 'N' },
        { frame: 100, kind: 'N' },
        { frame: 100, kind: 'C' },
      ])
    })
    const source = scheduler.createColdObservable(`aaaaaaaaa(a|`)
    const result = backoffDelay(BackoffStrategy.linear(1), { maxRetries: 100 }, scheduler)(source)
    scheduler.expectObservable(result).toBe('')
    scheduler.flush()
  })

  it('should reach max retries', () => {
    const scheduler = new TestScheduler(r => {
      expect(r.map(({ frame, notification: { kind } }: any) => ({ frame, kind }))).toEqual([
        { frame: 1, kind: 'N' },
        { frame: 12, kind: 'N' },
        { frame: 23, kind: 'N' },
        { frame: 30, kind: 'E' },
      ])
    })

    const source = scheduler.createColdObservable(`aaaaaaaaa(a|`)
    const result = backoffDelay(
      BackoffStrategy.linear(1),
      {
        maxRetries: 3,
      },
      scheduler,
    )(source)
    scheduler.expectObservable(result).toBe('')
    scheduler.flush()
  })

  it('should cap', () => {
    const scheduler = new TestScheduler(r => {
      expect(r.map(({ frame, notification: { kind } }: any) => ({ frame, kind }))).toEqual([
        { frame: 1, kind: 'N' },
        { frame: 12, kind: 'N' },
        { frame: 23, kind: 'N' },
        { frame: 34, kind: 'N' },
        { frame: 44, kind: 'N' },
        { frame: 54, kind: 'N' },
        { frame: 64, kind: 'N' },
        { frame: 74, kind: 'N' },
        { frame: 84, kind: 'N' },
        { frame: 94, kind: 'N' },
        { frame: 94, kind: 'C' },
      ])
    })

    const source = scheduler.createColdObservable(`aaaaaaaaa(a|`)
    const result = backoffDelay(
      BackoffStrategy.linear(1),
      {
        maxRetries: 100,
        maxDelayCap: Milliseconds.of(4),
      },
      scheduler,
    )(source)
    scheduler.expectObservable(result).toBe('')
    scheduler.flush()
  })

  it('should call onReachedMaxDelayCap', () => {
    const scheduler = new TestScheduler(() => true)
    const source = scheduler.createColdObservable(`aaaaaaaa(a|`)
    const onReachedMaxDelayCap = jest.fn()
    backoffDelay(
      BackoffStrategy.linear(1),
      {
        maxRetries: 100,
        maxDelayCap: Milliseconds.of(4),
        onReachedMaxDelayCap,
      },
      scheduler,
    )(source).subscribe()
    scheduler.flush()
    expect(onReachedMaxDelayCap).toHaveBeenCalledTimes(5)
  })

  it('should call onBackoff', () => {
    const scheduler = new TestScheduler(() => true)
    const source = scheduler.createColdObservable(`aa(a|`)
    const onBackoff = jest.fn()
    backoffDelay(
      BackoffStrategy.linear(10),
      {
        maxRetries: 100,
        onBackoff,
      },
      scheduler,
    )(source).subscribe()
    scheduler.flush()
    expect(onBackoff).toHaveBeenCalledTimes(3)
    expect(onBackoff.mock.calls).toEqual([[1, 10, 'a'], [2, 20, 'a'], [3, 30, 'a']])
  })
})
