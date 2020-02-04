/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable, Operator, ReplaySubject, Subject, Subscriber } from 'rxjs'
import { MonoTypeOperatorFunction } from 'rxjs/interfaces'
import { TeardownLogic } from 'rxjs/Subscription'
import { SelectTag, Tagged } from '../util'
/**
 * Given two hot observables, will concatenate them into another hot observable without dropping
 * elements. Elements of the second observable will be buffered until the first one completes.
 * @param {*} a a hot observable producing a finite number of elements
 * @param {*} b a hot observable producing a possibly infinite number of events
 */
export function concatHot<T>(a: Observable<T>, b: Observable<T>): Observable<T> {
  const notifier: Subject<void> = new Subject()
  const buffer: ReplaySubject<T> = new ReplaySubject()

  b.takeUntil(notifier).subscribe(buffer)
  // the complete handler is to stop buffering output from b as soon as a ends
  // all buffered elements will be emitted at once, then elements from b will just be passed through
  // the error handler is to prevent b being fed into buffer for all eternity if a fails
  return a
    .do({
      complete: () => notifier.next(),
      error: () => notifier.next(),
    })
    .concat(buffer)
    .concat(b)
}

/**
 * Describes a transform for each case of an ADT/tagged union
 */
export type AdtTransform<T extends Tagged, U> = {
  [P in T['type']]: (x: Observable<SelectTag<T, P>>) => Observable<U>
}

const combine = <T extends Tagged, U>(m: AdtTransform<T, U>) => (
  ts: Observable<T>,
): Observable<U> => ts.groupBy(_ => _.type).mergeMap(_ => _.pipe((m as any)[_.key]))

export const AdtTransform = {
  combine,
}

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
