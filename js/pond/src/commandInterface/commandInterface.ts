/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as t from 'io-ts'
import { Observable } from 'rxjs'
import { MultiplexedWebsocket } from '../eventstore/multiplexedWebsocket'
import { NodeId, Timestamp } from '../types'
import { CounterMap, DurationMap, GaugeMap, Loggers } from '../util'
import { mockCommandInterface } from './mockCommandInterface'
import { WebsocketCommandInterface } from './websocketCommandInterface'

/**
 * Message for all sorts of swarm control commands.
 *
 * This does not have a source, since the emitter might
 * not be a pond and we don't want to make up a source id.
 */
export const enum ControlCommandType {
  Backup = 'Backup',
  SetDebug = 'SetDebug',
}

export const SetDebug = t.readonly(
  t.type({
    type: t.literal(ControlCommandType.SetDebug),
    value: t.string,
    sources: t.readonlyArray(t.string),
    target: t.union([t.literal('shell'), t.undefined]),
  }),
)
export type SetDebug = t.TypeOf<typeof SetDebug>

export const Backup = t.readonly(
  t.type({
    type: t.literal(ControlCommandType.Backup),
  }),
)
export type Backup = t.TypeOf<typeof Backup>

export const ControlCommand = t.union([SetDebug, Backup])
export type ControlCommand = t.TypeOf<typeof ControlCommand>

export type LogLevel = 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR'
const logLevels: { [l in keyof Loggers]: LogLevel } = {
  debug: 'DEBUG',
  info: 'INFO',
  warn: 'WARNING',
  error: 'ERROR',
}
export const LogLevel = {
  of: (l: keyof Loggers) => logLevels[l],
}

/**
 * CommandInterface types and Interface
 */

export type AlertRequest = (message: string, time: Timestamp) => Promise<void>
export type HeartbeatRequest = () => Promise<void>
export type LoggingRequest = (level: string, tag: string, message: string) => Promise<void>
export type MetaRequest = (message: string) => Promise<void>
export type RunStatsRequest = (
  counters: DurationMap,
  durations: CounterMap,
  gauges: GaugeMap,
) => Promise<void>
export type SubscribeRequest = () => Observable<ControlCommand>

export interface CommandInterface {
  readonly sourceId: NodeId
  alert: AlertRequest
  heartbeat: HeartbeatRequest
  logging: LoggingRequest
  meta: MetaRequest
  runStats: RunStatsRequest
  subscribe: SubscribeRequest
}

const noopCommandInterface: CommandInterface = {
  sourceId: NodeId.of('noop'),
  alert: () => Promise.resolve(),
  heartbeat: () => Promise.resolve(),
  logging: () => Promise.resolve(),
  meta: () => Promise.resolve(),
  runStats: () => Promise.resolve(),
  subscribe: () => Observable.never(),
}

export const CommandInterface = {
  noop: noopCommandInterface,
  mock: mockCommandInterface,
  ws: (multiplexedWebsocket: MultiplexedWebsocket, sourceId: NodeId) =>
    new WebsocketCommandInterface(multiplexedWebsocket, sourceId),
}
