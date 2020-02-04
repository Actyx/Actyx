import { Observable, Scheduler, Subject, Subscriber } from 'rxjs'
import { OperatorFunction } from 'rxjs/interfaces'

type Todo<T> = Readonly<{
  consume: number
  from: Observable<T>
  to: Subscriber<T>
}>
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Entry = Todo<any> | null

const createSemaphore = (available: number): Semaphore => {
  if (available <= 0 || available > 512) {
    throw new Error('available should be between 1 and 512')
  }
  const input = new Subject<Entry>()
  const tokens = new Subject<null>()
  const sendTokens = (n: number): void => {
    for (let i = 0; i < n; i++) {
      tokens.next(null)
    }
  }
  const consumeTokens = (n: number): void => {
    for (let i = 0; i < n; i++) {
      input.next(null)
    }
  }
  const use = <T>(n: number): OperatorFunction<T, T> => {
    if (n < 1) {
      throw new Error('You must use at least one token')
    }
    const consume = Math.min(available, n)
    return (from: Observable<T>) =>
      new Observable<T>(to => {
        const entry: Todo<T> = {
          from,
          to,
          consume,
        }
        consumeTokens(consume - 1) // consume tokens without doing anything
        input.next(entry) // the op itself, consumes one token
      })
  }
  input
    .zip(tokens, entry => {
      if (entry !== null) {
        const { from, to, consume } = entry
        from.finally(() => sendTokens(consume)).subscribe(to)
      }
    })
    .observeOn(Scheduler.queue)
    .subscribe()
  sendTokens(available)
  return { use }
}

export type Semaphore = {
  /**
   * Use n tokens. n must be >=1.
   *
   * If n is larger than the total amount of tokens, it will be set to the maximum possible amount.
   *
   * The token will be held for the entire lifetime of the produced observable.
   */
  use: <T>(n: number) => OperatorFunction<T, T>
}

export const Semaphore = {
  /**
   * Create a semaphore with n available tokens.
   *
   * The returned object is mutable and should usually be stored in a central place as a singleton.
   */
  of: createSemaphore,
}
