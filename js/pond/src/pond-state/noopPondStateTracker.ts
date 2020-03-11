/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { noop } from '../util'
import { PondStateTracker } from './pond-state'

export const mkNoopPondStateTracker = (): PondStateTracker => ({
  hydrationStarted: () => '',
  hydrationFinished: noop,
  commandProcessingStarted: () => '',
  commandProcessingFinished: noop,
  eventsFromOtherSourcesProcessingStarted: () => '',
  eventsFromOtherSourcesProcessingFinished: noop,
  observe: () => Observable.never(),
})
