import { OperatorFunction } from '../../node_modules/rxjs/interfaces'
import { Scheduler, Observable, Operator, Subscriber } from '../../node_modules/rxjs'
import { TeardownLogic, Subscription } from '../../node_modules/rxjs/Subscription'

export function bufferOp<T>(
  bufferTimeSpan: number,
  maxBufferSize?: number,
): OperatorFunction<T, T[]> {
  return function bufferTimeOperatorFunction(source: Observable<T>) {
    return source.lift(
      new BufferTimeOperator<T>(bufferTimeSpan, maxBufferSize || Number.POSITIVE_INFINITY),
    )
  }
}

class BufferTimeOperator<T> implements Operator<T, T[]> {
  constructor(private bufferTimeSpan: number, private maxBufferSize: number) {}

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  call(subscriber: Subscriber<T[]>, source: any): TeardownLogic {
    return source.subscribe(
      new BufferTimeSubscriber(subscriber, this.bufferTimeSpan, this.maxBufferSize),
    )
  }
}

/**
 * We need this JSDoc comment for affecting ESDoc.
 * @ignore
 * @extends {Ignored}
 */
class BufferTimeSubscriber<T> extends Subscriber<T> {
  private buffer: T[] | null = null
  private closeAction: Subscription | null = null

  constructor(
    destination: Subscriber<T[]>,
    private bufferTimeSpan: number,
    private maxBufferSize: number,
  ) {
    super(destination)
  }

  protected _next(value: T) {
    if (this.buffer === null) {
      this.buffer = []
      this.add(
        (this.closeAction = Scheduler.async.schedule(() => {
          const b = this.buffer
          this.buffer = null
          if (b !== null && this.destination.next !== undefined) {
            this.destination.next(b)
          }
        }, this.bufferTimeSpan)),
      )
    }
    this.buffer.push(value)
    if (this.buffer.length >= this.maxBufferSize) {
      if (this.closeAction !== null) {
        const ca = this.closeAction
        this.closeAction = null
        ca.unsubscribe()
        this.remove(ca)
      }
      const b = this.buffer
      this.buffer = null
      if (this.destination.next !== undefined) {
        this.destination.next(b)
      }
    }
  }

  protected _complete() {
    const { buffer, destination } = this
    if (buffer !== null && destination !== undefined && destination.next !== undefined) {
      destination.next(buffer)
    }
    this.buffer = null
    super._complete()
  }
}
