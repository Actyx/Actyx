import { Observable, Subject } from 'rxjs'
import { OperatorFunction } from 'rxjs/interfaces'
import { concatMap, map } from 'rxjs/operators'
import { BufferSubject } from './BufferSubject'
import { dropWhileBusy } from './dropWhileBusy'

const setup = (pipeline?: OperatorFunction<[number, () => void], number>) => {
  const input = new Subject<number>()
  const result: number[] = []
  const progress = new BufferSubject<() => void>()
  const step = () => new Promise(resolve => progress.next(resolve))
  const done = new Promise(resolve =>
    input
      .pipe(
        dropWhileBusy(),
        pipeline
          ? pipeline
          : concatMap(([value, ready]) =>
              progress
                .take(1)
                .do(() => ready())
                .do(p => setTimeout(p, 0)) // to ensure that test waits for results
                .mapTo(value),
            ),
      )
      .subscribe(
        value => result.push(value),
        () => {
          result.push(54)
          resolve()
        },
        () => {
          result.push(42)
          resolve()
        },
      ),
  )
  return { input, step, result, done }
}

describe('dropWhileBusy', () => {
  it('must run normally', async () => {
    const { input, step, result } = setup()
    input.next(1)
    await step()
    input.next(2)
    await step()
    input.next(3)
    await step()
    input.complete()
    expect(result).toEqual([1, 2, 3, 42])
  })
  it('must run first and last', async () => {
    const { input, step, result } = setup()
    input.next(1)
    input.next(2)
    await step()
    await step()
    expect(result).toEqual([1, 2])
  })
  it('must drop elements between first and last', async () => {
    const { input, step, result } = setup()
    Observable.of(1, 2, 3, 4, 5).subscribe(input)
    await step()
    await step()
    expect(result).toEqual([1, 5, 42])
  })
  it('must pass along errors', async () => {
    const { input, done, result } = setup()
    Observable.of(1, 2, 3)
      .concat(Observable.throw(54))
      .subscribe(input)
    await done
    expect(result).toEqual([54])
  })
  it('must support synchronous acknowledgement', async () => {
    const { input, result, done } = setup(
      map(([value, ready]) => {
        ready()
        return value
      }),
    )
    Observable.of(1, 2, 3, 4, 5).subscribe(input)
    await done
    expect(result).toEqual([1, 5, 42])
  })
  it('must pass along errors with synchronous acknoledgement', async () => {
    const { input, done, result } = setup(
      map(([value, ready]) => {
        ready()
        return value
      }),
    )
    Observable.of(1, 2, 3)
      .concat(Observable.throw(54))
      .subscribe(input)
    await done
    expect(result).toEqual([54])
  })
})
