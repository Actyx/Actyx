import { Subject } from 'rxjs/Subject'
import { SubjectSubscription } from 'rxjs/SubjectSubscription'
import { Subscriber } from 'rxjs/Subscriber'
import { Subscription } from 'rxjs/Subscription'
import { ObjectUnsubscribedError } from 'rxjs/util/ObjectUnsubscribedError'

export class BufferSubject<T> extends Subject<T> {
  private _events: T[] = []

  next(value: T): void {
    if (this.observers.length === 0) {
      this._events.push(value)
    } else {
      super.next(value)
    }
  }

  /** @deprecated internal use only */ _subscribe(subscriber: Subscriber<T>): Subscription {
    const _events = this._events
    let subscription: Subscription

    if (this.closed) {
      throw new ObjectUnsubscribedError()
    } else if (this.hasError) {
      subscription = Subscription.EMPTY
    } else if (this.isStopped) {
      subscription = Subscription.EMPTY
    } else {
      this.observers.push(subscriber)
      subscription = new SubjectSubscription(this, subscriber)
    }

    let i = 0
    for (; i < _events.length && !subscriber.closed; i += 1) {
      subscriber.next(_events[i])
    }
    _events.splice(0, i)

    if (this.hasError) {
      subscriber.error(this.thrownError)
    } else if (this.isStopped) {
      subscriber.complete()
    }

    return subscription
  }
}
