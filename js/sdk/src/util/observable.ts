/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable, Operator, Subscriber } from 'rxjs'
import { MonoTypeOperatorFunction } from 'rxjs/interfaces'
import { TeardownLogic } from 'rxjs/Subscription'
import { CancelSubscription } from '../types'
import { noop } from './typescript'

/**
 * Just like takeWhile but will also emit the element on which the predicate has fired
 * @param predicate the predicate for this operator
 */
export const takeWhileInclusive = <T>(
  predicate: (value: T, index: number) => boolean,
): MonoTypeOperatorFunction<T> => (source: Observable<T>) =>
  source.lift(new TakeWhileInclusiveOperator(predicate))

class TakeWhileInclusiveOperator<T> implements Operator<T, T> {
  constructor(private predicate: (value: T, index: number) => boolean) {}

  call(subscriber: Subscriber<T>, source: any): TeardownLogic {
    return source.subscribe(new TakeWhileInclusiveSubscriber(subscriber, this.predicate))
  }
}

/**
 * We need this JSDoc comment for affecting ESDoc.
 * @ignore
 * @extends {Ignored}
 */
class TakeWhileInclusiveSubscriber<T> extends Subscriber<T> {
  private index: number = 0

  constructor(destination: Subscriber<T>, private predicate: (value: T, index: number) => boolean) {
    super(destination)
  }

  protected _next(value: T): void {
    const destination = this.destination

    let result: boolean
    try {
      result = this.predicate(value, this.index++)
    } catch (err) {
      if (destination.error) destination.error(err)
      return
    }
    this.nextOrComplete(value, result)
  }

  private nextOrComplete(value: T, predicateResult: boolean): void {
    const destination = this.destination
    if (destination.next) destination.next(value)
    if (!predicateResult) {
      this.complete()
    }
  }
}

export const omitObservable = <T>(
  stoppedByError: ((err: unknown) => void) | undefined,
  callback: (newVal: T) => void,
  obs: Observable<T>,
): CancelSubscription => {
  try {
    // Not passing an error callback seems to cause bad behavior with RXjs internally
    const sub = obs.subscribe(
      callback,
      typeof stoppedByError === 'function' ? stoppedByError : noop,
    )
    return sub.unsubscribe.bind(sub)
  } catch (err) {
    stoppedByError && stoppedByError(err)
    return noop
  }
}
