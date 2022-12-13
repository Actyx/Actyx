import { lastValueFrom, Subject, toArray, catchError, NEVER, of } from '../../node_modules/rxjs'
import { bufferOp } from './bufferOp'

describe('bufferOp', () => {
  it('should pass along values', async () => {
    const s = new Subject<number>()
    const prom = lastValueFrom(s.pipe(bufferOp(5), toArray()))

    s.next(1)
    s.next(2)
    await new Promise((res) => setTimeout(res, 100))
    s.next(3)
    s.next(4)
    s.next(5)
    await new Promise((res) => setTimeout(res, 100))
    s.complete()

    expect(await prom).toEqual([
      [1, 2],
      [3, 4, 5],
    ])
  })

  it('should pass along errors', async () => {
    const s = new Subject<number>()
    const prom = lastValueFrom(
      s.pipe(
        bufferOp(5),
        catchError((e) => [`${e}`]),
        toArray(),
      ),
    )

    s.next(1)
    s.error(new Error('hello'))

    expect(await prom).toEqual(['Error: hello'])
  })

  it('should pass along error after value', async () => {
    const s = new Subject<number>()
    const prom = lastValueFrom(
      s.pipe(
        bufferOp(5),
        catchError((e) => [`${e}`]),
        toArray(),
      ),
    )

    s.next(1)
    await new Promise((res) => setTimeout(res, 100))
    s.error(new Error('hello'))

    expect(await prom).toEqual([[1], 'Error: hello'])
  })

  it('should clean up resources', async () => {
    const s = new Subject()
    const call: string[] = []
    s.subscribe({
      next: () => call.push('next'),
      error: () => call.push('error'),
      complete: () => call.push('complete'),
    })
    const prom = lastValueFrom(s)

    const sub = NEVER.pipe(bufferOp(5)).subscribe(s)
    sub.unsubscribe()
    expect(call).toEqual([])
    of(42).subscribe(s)
    expect(await prom).toEqual(42)
    expect(call).toEqual(['next', 'complete'])
  })
})
