/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { FishType, InitialState, OnCommand, OnEvent, OnStateChange, Semantics } from '..'

export type State = ReadonlyArray<number> // all received events

export type Command = ReadonlyArray<number> // emitted as individual events

export type Event = number // payload of an event is just a number

export type Config = Readonly<{
  /**
   * Max delay between event bursts
   */
  maxDelayMs: number
  /**
   * Maximum number of events per burst (inclusive, minimum is 1)
   */
  maxEvents: number
  /**
   * Random seed
   */
  seed?: string
}>

export const eventStressFishBuilder = (
  semantics: Semantics,
  is: InitialState<State>,
): FishType<Command, Event, State> => {
  const onEvent: OnEvent<State, Event> = (state, event) => [...state, event.payload]

  const onCommand: OnCommand<State, Command, Event> = (_, command) => command

  const initialState: InitialState<State> = is

  return FishType.of<State, Command, Event, State>({
    semantics,
    initialState,
    onEvent,
    onCommand,
    onStateChange: OnStateChange.publishPrivateState(),
  })
}

const singleSourceInitialState: InitialState<State> = () => ({ state: [] })

export const eventStressFish = (semantics: Semantics): FishType<Command, Event, State> =>
  eventStressFishBuilder(semantics, singleSourceInitialState)
