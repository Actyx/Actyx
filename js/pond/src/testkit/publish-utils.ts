/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { BehaviorSubject, Observable, ReplaySubject, Subject } from 'rxjs'
import { FishName, FishType, FishTypeImpl, PondObservables } from '../types'

export class TestPondObservables<S> implements PondObservables<S> {
  state: {
    [param: string]: {
      [param: string]: {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        state: Subject<any>
      }
    }
  } = {}
  subject: Subject<S> = new ReplaySubject<S>(1)

  put = <C, E, P>(fishType: FishType<C, E, P>, name: FishName, publicState: P): void => {
    if (!this.state[fishType.semantics]) {
      this.state[fishType.semantics] = {}
    }
    const sem = this.state[fishType.semantics]
    if (!sem[name]) {
      const state = new BehaviorSubject(publicState)
      sem[name] = { state }
    } else {
      sem[name].state.next(publicState)
    }
  }

  putSelf = (s: S): void => {
    this.subject.next(s)
  }

  observe = <C, E, P>(fish: FishType<C, E, P>, fishName: string): Observable<P> => {
    if (!this.state[fish.semantics]) {
      throw new Error(`trying to observe uninitialized fish type "${fish.semantics}"`)
    }
    if (!this.state[fish.semantics][fishName]) {
      throw new Error(`trying to observe uninitialized fish "${fish.semantics}/${name}"`)
    }
    return this.state[fish.semantics][fishName].state
  }

  observeSelf = (): Observable<S> => {
    return this.subject
  }
}

export function firstPublished<S, C, E, P>(
  pond: TestPondObservables<S>,
  fishType: FishType<C, E, P>,
  state: S,
  filter: (publicState: P) => boolean = () => true,
): Promise<P> {
  pond.putSelf(state)
  return FishTypeImpl.downcast(fishType)
    .onStateChange(pond)
    .concatMap(e => (e.type === 'publish' ? [e.state] : []))
    .filter(filter)
    .take(1)
    .toPromise()
}

export function noPublished<S, C, E, P>(
  pond: TestPondObservables<S>,
  fishType: FishType<C, E, P>,
  state: S,
  filter: (publicState: P) => boolean = () => true,
): Promise<boolean> {
  pond.putSelf(state)
  return FishTypeImpl.downcast(fishType)
    .onStateChange(pond)
    .concatMap(e => (e.type === 'publish' ? [e.state] : []))
    .filter(filter)
    .map(x => {
      throw new Error(JSON.stringify(x))
    })
    .race(Observable.of(true).delay(100))
    .toPromise()
}

export function firstCommand<S, C, E, P>(
  pond: TestPondObservables<S>,
  fishType: FishType<C, E, P>,
  state: S,
  filter: (command: C) => boolean = () => true,
): Promise<C> {
  pond.putSelf(state)
  return FishTypeImpl.downcast(fishType)
    .onStateChange(pond)
    .concatMap(e => (e.type === 'sendSelfCommand' ? [e.command] : []))
    .filter(filter)
    .take(1)
    .toPromise()
}

export function noCommand<S, C, E, P>(
  pond: TestPondObservables<S>,
  fishType: FishType<C, E, P>,
  state: S,
  filter: (command: C) => boolean = () => true,
): Promise<boolean> {
  pond.putSelf(state)
  return FishTypeImpl.downcast(fishType)
    .onStateChange(pond)
    .concatMap(e => (e.type === 'sendSelfCommand' ? [e.command] : []))
    .filter(filter)
    .map(x => {
      throw new Error(JSON.stringify(x))
    })
    .race(Observable.of(true).delay(100))
    .toPromise()
}
