/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { CommandResult, Envelope, SourceId } from '..'
import { FishTypeImpl } from '../types'
import { TestCommandExecutor, TestCommandExecutorConfig, TestResult } from './testCommandExecutor'

const defaultConfig = {}

const mkTestFunctions = <C, E, S>(
  fish: FishTypeImpl<S, C, E, any>,
  config: Partial<FishTestFunctionsConfig>,
): FishTestFunctions<C, E, S> => {
  const config1 = { ...defaultConfig, ...config }
  const commandExecutor = TestCommandExecutor(config1.commandExecutorConfig || {})
  const onEvent = (s: S, e: Envelope<E>) => {
    const state1 = s
    return fish.onEvent(state1, e)
  }
  const onCommand = (s: S, c: C): TestResult<any, ReadonlyArray<E>> => {
    const cr: CommandResult<E> = fish.onCommand(s, c)
    const res = CommandResult.fold<E, TestResult<any, ReadonlyArray<E>>>(cr)({
      sync: events => TestResult.of(events, []),
      async: x => commandExecutor(x),
      none: () => {
        throw new Error()
      },
    })
    return res
  }
  const initialState = (fishName: string, sourceId: SourceId): S => {
    const state = fish.initialState(fishName, sourceId).state
    return state
  }
  return {
    fish,
    initialState,
    onEvent,
    onCommand,
  }
}

export type FishTestFunctions<C, E, S> = {
  fish: FishTypeImpl<S, C, E, any>
  /**
   * Produces just the initial state of the wrapped fish, deep-frozen depending on config
   */
  initialState: (fishName: string, sourceId: SourceId) => S
  /**
   * Calls onEvent on the wrapped fish and returns just the state
   */
  onEvent: (s: S, e: Envelope<E>) => S
  /**
   * Calls onCommand on the wrapped fish and returns effects and events as a TestResult
   */
  onCommand: (s: S, c: C) => TestResult<any, ReadonlyArray<E>>
}

export type FishTestFunctionsConfig = Readonly<{
  commandExecutorConfig?: Partial<TestCommandExecutorConfig>
}>

/**
 * Creates two functions onEvent and onCommand that can be used to test a fish
 * with both sync and async events.
 */
export const FishTestFunctions = {
  of: mkTestFunctions,
}
