import { OperatorFunction, Observable, Subscription, asyncScheduler } from '../../node_modules/rxjs'

export const bufferOp =
  <T>(bufferTimeSpan: number, maxBufferSize?: number): OperatorFunction<T, T[]> =>
  (source: Observable<T>) =>
    new Observable((subscriber) => {
      let buffer: T[] | null = null
      let timer: Subscription | null = null
      const maxBuf = maxBufferSize || Infinity

      const startBuffer = (_b: T[] | null): _b is T[] => {
        if (buffer === null) {
          buffer = []
          const sub = asyncScheduler.schedule(emit, bufferTimeSpan)
          subscriber.add(sub)
          timer = sub
        }
        return true
      }

      const cancelTimer = () => {
        if (timer !== null) {
          timer.unsubscribe()
          subscriber.remove(timer)
          timer = null
        }
      }

      const emit = () => {
        const b = buffer
        buffer = null
        if (b != null) {
          cancelTimer()
          subscriber.next(b)
        }
      }

      const sub = source.subscribe({
        next: (value) => {
          if (!startBuffer(buffer)) return
          buffer.push(value)
          if (buffer.length >= maxBuf) emit()
        },
        error: (err) => subscriber.error(err),
        complete: () => {
          emit()
          subscriber.complete()
        },
      })
      sub.add(() => subscriber.error(new Error('cancelled')))

      return sub
    })
