/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Milliseconds } from '@actyx/sdk'
import { BehaviorSubject, Observable } from 'rxjs'
import { FishName, Semantics } from '../types'
import { Loggers } from '../util'
import { PondState, PondStateTracker } from './pond-state'

export const mkInitialState = (): PondState => ({
  hydration: {
    numBeingProcessed: 0,
    fish: {},
  },
  commands: {
    numBeingProcessed: 0,
    fish: {},
  },
  eventsFromOtherSources: {
    numBeingProcessed: 0,
    fish: {},
  },
})

const mkKeyWithTimestamp = (now: () => number) => (
  fishSemantics: Semantics,
  fishName: FishName,
): string => `${fishSemantics}:${fishName}:${now()}`

export const mkPondStateTracker = (log: Loggers, now?: () => Milliseconds): PondStateTracker => {
  const state: PondState = mkInitialState()

  const stateSubject = new BehaviorSubject<PondState>(state)

  const mkKey = mkKeyWithTimestamp(now || Milliseconds.now)
  const notifySubscribers = (): void => stateSubject.next(state)

  const processStarted = (reg: keyof PondState) => (
    fishSemantics: Semantics,
    fishName: FishName,
  ): string => {
    const key = mkKey(fishSemantics, fishName)
    state[reg].fish[key] = true
    state[reg].numBeingProcessed += 1
    notifySubscribers()

    return key
  }

  const processingFinished = (reg: keyof PondState, errorMsg: string) => (key: string): void => {
    if (state[reg].fish[key] === undefined) {
      log.warn(errorMsg, key)
      return
    }

    delete state[reg].fish[key]
    state[reg].numBeingProcessed -= 1
    notifySubscribers()
  }

  return {
    hydrationStarted: processStarted('hydration'),
    hydrationFinished: processingFinished(
      'hydration',
      'Hydration ended for an unknown fish with key: %s.',
    ),
    commandProcessingStarted: processStarted('commands'),
    commandProcessingFinished: processingFinished(
      'commands',
      'Command processing finished for an unknown command with key: %s.',
    ),
    eventsFromOtherSourcesProcessingStarted: processStarted('eventsFromOtherSources'),
    eventsFromOtherSourcesProcessingFinished: processingFinished(
      'eventsFromOtherSources',
      'Events from other sources processing finished for an unknown chunk with key: %s.',
    ),
    observe: (): Observable<PondState> => {
      return stateSubject.asObservable()
    },
  }
}
