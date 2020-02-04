import { Observable } from 'rxjs'
import { Semaphore } from './rxSemaphore'

describe('rxSemaphore', () => {
  const getConcurrencyLevel = async (
    sem: Semaphore,
    requests: number,
    tokensPerRequest: number,
    maxAllowed: number,
  ): Promise<number> => {
    let inProgress = 0
    let maxInProgress = 0
    const block = Observable.of(undefined)
      .do(() => {
        inProgress++
        maxInProgress = Math.max(maxInProgress, inProgress)
        expect(inProgress).toBeLessThanOrEqual(maxAllowed)
      })
      .delay(20)
      .do(() => {
        inProgress--
      })
      .pipe(sem.use(tokensPerRequest))
    await Observable.range(0, requests)
      .mergeMap(() => block)
      .toArray()
      .toPromise()
    return maxInProgress
  }

  it('should limit requests', async () => {
    const sem = Semaphore.of(3)
    const c = await getConcurrencyLevel(sem, 10, 1, 3)
    expect(c).toEqual(3)
  })

  it('should work with use != 1', async () => {
    const sem = Semaphore.of(8)
    const c = await getConcurrencyLevel(sem, 10, 2, 4)
    expect(c).toEqual(4)
  })

  it('properly deal with observables that fail', async () => {
    // failing observables should consume all the tokens
    const sem = Semaphore.of(8)
    const fail = Observable.of(undefined)
      .delay(20)
      .concat(Observable.throw('BOOM'))
      .pipe(sem.use(2)) // each request uses 2 tokens
    const error = await Observable.range(0, 10)
      .mergeMap(() => fail)
      .toArray()
      .catch(e => Observable.of(e))
      .toPromise()
    expect(error).toEqual('BOOM')
    // this will only work if the tokens are available again
    const c = await getConcurrencyLevel(sem, 10, 2, 4)
    expect(c).toEqual(4)
  })

  it('properly deal with observables that are unsubscribed', async () => {
    // never observables should consume all the tokens
    const sem = Semaphore.of(8)
    const never = Observable.never<void>().pipe(sem.use(2)) // each request uses 2 tokens
    const subscriptions = [1, 2, 3, 4, 5].map(() => never.subscribe())
    await Observable.timer(50).toPromise()
    subscriptions.forEach(s => {
      s.unsubscribe()
    })
    // this will only work if the tokens are available again
    const c = await getConcurrencyLevel(sem, 10, 2, 4)
    expect(c).toEqual(4)
  })

  it('should handle the GC scenario', async () => {
    const sem = Semaphore.of(3)
    let fetchInProgress = 0
    let gcInProgress = 0
    const fetch = Observable.of(undefined)
      .do(() => {
        fetchInProgress++
        expect(gcInProgress).toEqual(0)
      })
      .delay(10)
      .do(() => fetchInProgress--)
      .pipe(sem.use(1)) // fetch takes 1 slot
    const gc = Observable.of(undefined)
      .do(() => {
        gcInProgress++
        expect(gcInProgress).toEqual(1)
        expect(fetchInProgress).toEqual(0)
      })
      .delay(20)
      .do(() => gcInProgress--)
      .pipe(sem.use(3)) // gc takes all slots
    await Observable.from([fetch, fetch, fetch, fetch, gc, fetch, fetch])
      .mergeMap(x => x)
      .toArray()
      .toPromise()
  })
})
