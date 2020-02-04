import { mergeDeepRight } from 'ramda'
import { Observable } from 'rxjs'
import { MonoTypeOperatorFunction } from 'rxjs/interfaces'
import { retryWhen } from 'rxjs/operators'
import { IScheduler } from 'rxjs/Scheduler'
import { Milliseconds } from '../types'

export type BackoffStrategy = (retryCount: number) => Milliseconds

export const BackoffStrategy = {
  linear: (initialDelay: number = 1000, random: () => number = () => 0) => (
    retryCount: number,
  ): Milliseconds => Milliseconds.of((retryCount + 1) * initialDelay + random()),
  exponential: (initialDelay: number = 1000) => (retryCount: number): Milliseconds =>
    Milliseconds.of(Math.pow(2, retryCount) * initialDelay),
}

export type BackoffStrategyConfig = {
  maxDelayCap?: Milliseconds
  maxRetries: number
  name?: string
  onReachedMaxDelayCap?: (maxDelayCap: Milliseconds) => void
  onBackoff?: (retryCounter: number, delay: Milliseconds, err: Error) => void
}

const defaultBackoffStrategyConfig = {
  capDelay: 300 * 1000,
  maxRetries: 1024 * 1024,
}

export const backoffDelay = (
  backoffStrategy: BackoffStrategy,
  { onBackoff, onReachedMaxDelayCap, maxRetries, maxDelayCap }: BackoffStrategyConfig,
  scheduler?: IScheduler,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
) => (source$: Observable<any>) =>
  Observable.range(0, maxRetries + 1)
    .zip(source$, (retryCount, err) => ({ retryCount, err }))
    .mergeMap(({ retryCount, err }) => {
      if (retryCount === maxRetries) {
        return Observable.throw(err)
      }

      let delay = backoffStrategy(retryCount)
      if (maxDelayCap && maxDelayCap < delay) {
        delay = maxDelayCap
        onReachedMaxDelayCap && onReachedMaxDelayCap(delay)
      }

      return onBackoff
        ? Observable.timer(delay, scheduler).do(() => onBackoff(retryCount + 1, delay, err))
        : Observable.timer(delay, scheduler)
    })

export const retryOnErrorWithBackoff = <T>(
  source$: Observable<T>,
  backoffStrategy: BackoffStrategy = BackoffStrategy.linear(),
  config: Partial<BackoffStrategyConfig> = {},
  scheduler?: IScheduler,
) =>
  source$.retryWhen(
    // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
    // @ts-ignore FIXME Ramda types
    backoffDelay(backoffStrategy, mergeDeepRight(defaultBackoffStrategyConfig, config), scheduler),
  )

export const retryOnErrorWithBackoffOp = <T>(
  backoffStrategy: BackoffStrategy = BackoffStrategy.linear(),
  config: Partial<BackoffStrategyConfig> = {},
  scheduler?: IScheduler,
): MonoTypeOperatorFunction<T> =>
  retryWhen(
    backoffDelay(
      backoffStrategy,
      // TODO, as above.
      mergeDeepRight(defaultBackoffStrategyConfig, config) as BackoffStrategyConfig,
      scheduler,
    ),
  )
