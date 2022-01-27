/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable, OperatorFunction } from '../../node_modules/rxjs'
import { CancelSubscription } from '../types'
import { noop } from './typescript'

/**
 * Just like takeWhile but will also emit the element on which the predicate has fired
 * @param predicate the predicate for this operator
 */
export const takeWhileInclusive =
  <T>(predicate: (value: T, index: number) => boolean): OperatorFunction<T, T> =>
  (source: Observable<T>) =>
    new Observable((subscriber) => {
      let index = 0
      return source.subscribe({
        next: (value) => {
          const result = predicate(value, index)
          index += 1
          subscriber.next(value)
          if (!result) subscriber.complete()
        },
      })
    })

export const omitObservable = <T>(
  stoppedByError: ((err: unknown) => void) | undefined,
  callback: (newVal: T) => void,
  obs: Observable<T>,
): CancelSubscription => {
  try {
    // Not passing an error callback seems to cause bad behavior with RXjs internally
    const sub = obs.subscribe({
      next: callback,
      error: stoppedByError,
    })
    return () => sub.unsubscribe()
  } catch (err) {
    stoppedByError && stoppedByError(err)
    return noop
  }
}
