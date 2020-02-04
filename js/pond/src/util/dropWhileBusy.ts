import { Operator, pipe, Subject, Subscriber } from 'rxjs'
import { InnerSubscriber } from 'rxjs/InnerSubscriber'
import { OperatorFunction } from 'rxjs/interfaces'
import { map } from 'rxjs/operators'
import { OuterSubscriber } from 'rxjs/OuterSubscriber'
import { TeardownLogic } from 'rxjs/Subscription'
import { subscribeToResult } from 'rxjs/util/subscribeToResult'

type Ready = () => void

/**
 * This combinator contains a buffer of size 1 and emits a readiness callback
 * with every element downstream: while readiness has not yet been signaled,
 * further elements will be buffered, dropping the old buffer content.
 *
 * Since all downstream processing is deferred using `setTimeout(..., 0)`,
 * this combinator will respond to increasing system load by dropping more
 * intermediate elements, i.e. throttling the rate that goes downstream.
 *
 * It is important to note that of a throttled run of elements, the first and the
 * last will always be passed downstream.
 */
export const dropWhileBusy: <T>() => OperatorFunction<T, [T, Ready]> = () => source =>
  source.lift(new DropWhileBusyOperator())

/**
 * This combinator contains a buffer of size 1 and retains the new element when
 * incoming while the buffer is full. All downstream processing is deferred using
 * `setTimeout(..., 0)` and the buffer is emptied once downstream `next()` has
 * returned. This leads to dropping intermediate updates that come in very
 * quickly and it also will throttle the downstream rate during periods of high
 * system load by dropping elements.
 *
 * It is important to note that of a throttled run of elements, the first and the
 * last will always be passed downstream.
 */
export const dropWhileBusySync: <T>() => OperatorFunction<T, T> = () =>
  pipe(
    dropWhileBusy(),
    map(([value, ready]) => {
      ready()
      return value
    }),
  )

class DropWhileBusyOperator<T> implements Operator<T, T> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  call(subscriber: Subscriber<T>, source: any /* yes, that’s the interface */): TeardownLogic {
    return source.subscribe(new DropWhileBusySubscriber(subscriber))
  }
}

class DropWhileBusySubscriber<T> extends OuterSubscriber<T, [T, Ready]> {
  private buffer: T | undefined = undefined
  private ready = new Subject<void>()
  private beReady = () => {
    this.ready.next()
  }
  private running = false

  constructor(destination: Subscriber<T>) {
    super(destination)
    this.add(subscribeToResult(this, this.ready))
  }

  protected _next(value: T): void {
    this.buffer = value
    return !this.running ? this.dispatch() : undefined
  }

  protected _complete(): void {
    return !this.running ? super._complete() : undefined
  }

  notifyNext(
    _outerValue: T,
    _innerValue: [T, Ready],
    _outerIndex: number,
    _innerIndex: number,
    _innerSub: InnerSubscriber<T, [T, Ready]>,
  ): void {
    this.dispatch()
  }

  private dispatch(): void {
    const value = this.buffer
    this.buffer = undefined

    if (value !== undefined) {
      this.running = true
      setTimeout(() => {
        if (this.destination.next === undefined) {
          return
        }
        this.destination.next([value, this.beReady])
      }, 0)
    } else {
      this.running = false
      if (this.isStopped && this.destination.complete !== undefined) {
        setTimeout(() => {
          super._complete()
        }, 0)
      }
    }
  }
}
