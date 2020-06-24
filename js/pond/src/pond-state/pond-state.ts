/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'

export type FishProcessInfo = {
  numBeingProcessed: number
  fish: {
    [semantics: string]: true | undefined
  }
}

export type PondState = {
  hydration: FishProcessInfo
  commands: FishProcessInfo
  eventsFromOtherSources: FishProcessInfo
}

const isHydrating = (state: PondState): boolean => state.hydration.numBeingProcessed > 0
const isProcessingCommands = (state: PondState): boolean => state.commands.numBeingProcessed > 0
const isProcessingEventsFromOtherSources = (state: PondState): boolean =>
  state.eventsFromOtherSources.numBeingProcessed > 0
const isBusy = (state: PondState): boolean =>
  isHydrating(state) || isProcessingCommands(state) || isProcessingEventsFromOtherSources(state)

export const PondState = {
  isHydrating,
  isProcessingCommands,
  isProcessingEventsFromOtherSources,
  isBusy,
}

export type PondStateTracker = {
  observe(): Observable<PondState>

  /**
   * Returns key of the record
   */
  hydrationStarted(fishSemantics: string, fishName: string): string

  hydrationFinished(key: string): void

  /**
   * Returns key of the record
   */
  commandProcessingStarted(fishSemantics: string, fishName: string): string

  commandProcessingFinished(key: string): void

  /**
   * Returns key of the record
   */
  eventsFromOtherSourcesProcessingStarted(fishSemantics: string, fishName: string): string

  eventsFromOtherSourcesProcessingFinished(key: string): void
}
