import { Observable } from 'rxjs'
import { Observer } from 'rxjs/Observer'
import { SubscriptionManager } from './subscriptionManager'

type State = string[]
type Pond = string
type Effect = string

type OnStateChange = (
  state: State,
) => { readonly name: string; readonly create: (pond: Pond) => Observable<Effect> }[]

describe('subscriptionManager', () => {
  const calledStates: State[] = []
  let unsubscribe: number

  beforeEach(() => {
    calledStates.length = 0
    unsubscribe = 0
  })

  const onStateChange: OnStateChange = (state: State) => {
    calledStates.push(state)
    const activePipelines: string[] = state
    const handlers = activePipelines.map(name => ({
      name,
      create: () =>
        Observable.create((observer: Observer<string>) => {
          observer.next(name)
          observer.complete()
          return () => {
            unsubscribe += 1
          }
        }),
    }))
    return handlers
  }

  it('should create pipelines depending on state', () => {
    const sm = SubscriptionManager.of('', onStateChange)
    const states: State[] = [[], ['a'], ['a', 'b'], ['b'], []]
    return sm
      .manage(Observable.from(states))
      .toArray()
      .toPromise()
      .then((effects: Effect[]) => {
        expect(effects).toEqual(['a', 'b'])
        expect(unsubscribe).toEqual(2)
        expect(calledStates).toEqual(states)
      })
  })

  it('should resubscribe if a pipeline appears twice', () => {
    const sm = SubscriptionManager.of('', onStateChange)
    const states: State[] = [[], ['a'], [], ['a'], []]
    return sm
      .manage(Observable.from(states))
      .toArray()
      .toPromise()
      .then((effects: Effect[]) => {
        expect(effects).toEqual(['a', 'a'])
        expect(unsubscribe).toEqual(2)
        expect(calledStates).toEqual(states)
      })
  })
})
