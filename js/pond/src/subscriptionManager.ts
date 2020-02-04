/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable, Subject } from 'rxjs'

export type Factory<Pond, Effect> = Readonly<{
  name: string
  create: (pond: Pond) => Observable<Effect>
}>

export type OnStateChange<State, Pond, Effect> = (
  state: State,
) => ReadonlyArray<Factory<Pond, Effect>>

type Acc<Effect> = Readonly<{
  /**
   * Currently active names
   */
  names: ReadonlyArray<string>
  /**
   * Combined observable of all observables that were created in this state transition
   */
  toCreate: Observable<Effect>
}>

export class SubscriptionManager<State, Pond, Effect> {
  // current generations, for checking if a command is still active
  generations: { [name: string]: number } = {}

  private constructor(
    private pond: Pond,
    private onStateChange: OnStateChange<State, Pond, Effect>,
  ) {}

  /**
   * Creates a subscription manager, which is a stateful observable transform that takes care of
   * subscribing to and unsubscribing from named pipelines based on an opaque state.
   *
   * @param {*} pond a thing that somehow allows getting observables
   * @param {*} onStateChange a function that produces a list of named observable factories
   */
  static of<State, Pond, Effect>(
    pond: Pond,
    onStateChange: OnStateChange<State, Pond, Effect>,
  ): SubscriptionManager<State, Pond, Effect> {
    return new SubscriptionManager(pond, onStateChange)
  }

  manage(states: Observable<State>): Observable<Effect> {
    // subject that is used to publish deletes, where cancellation is performed using takeUntil()
    const deleteSubject: Subject<string> = new Subject()

    // aggregate function that publishes on the deleteSubject as a side effect
    const combine: (previous: Acc<Effect>, current: ReadonlyArray<Factory<Pond, Effect>>) => any = (
      previous: Acc<Effect>,
      current: ReadonlyArray<Factory<Pond, Effect>>,
    ) => {
      const previousNames = previous.names
      const currentNames = current.map(x => x.name)
      // new pipelines
      const newPipelines = current.filter(x => !previousNames.includes(x.name))
      const toCreate: Observable<Effect> = Observable.merge(
        ...newPipelines.map(c => {
          const name = c.name
          this.generations[name] = 0
          return c
            .create(this.pond)
            .concat(Observable.never<Effect>()) // must not emit 'delete' effect before deleteSubject says so
            .takeUntil(deleteSubject.filter(x => x === name))
            .finally(() => {
              delete this.generations[name]
            })
        }),
      )
      // delete pipelines that are no longer in the current set of names
      previousNames.forEach(name => {
        if (!currentNames.includes(name)) {
          deleteSubject.next(name)
        }
      })
      return { names: currentNames, toCreate }
    }

    return states
      .map(state => this.onStateChange(state))
      .scan(combine, { names: [], toCreate: Observable.empty<any>() })
      .mergeMap(acc => acc.toCreate)
  }
}
