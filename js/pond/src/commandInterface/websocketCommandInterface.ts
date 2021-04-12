/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as t from 'io-ts'
import { MultiplexedWebsocket, validateOrThrow } from '../eventstore/multiplexedWebsocket'
import { NodeId, Timestamp } from '../types'
import { CounterMap, DurationMap, GaugeMap } from '../util'
import {
  AlertRequest,
  CommandInterface,
  ControlCommand,
  HeartbeatRequest,
  LoggingRequest,
  MetaRequest,
  RunStatsRequest,
  SubscribeRequest,
} from './commandInterface'

const enum RequestTypes {
  Commands = '/ax/misc/commands',
  Logging = '/ax/misc/logging',
}

const Subscribe = t.readonly(
  t.type({
    type: t.literal('subscribe'),
  }),
)
type Subscribe = t.TypeOf<typeof Subscribe>

const AlertData = t.readonly(
  t.type({
    type: t.literal('alert'),
    message: t.string,
    time: Timestamp.FromNumber,
  }),
)
type AlertData = t.TypeOf<typeof AlertData>

const HeartbeatData = t.readonly(
  t.type({
    type: t.literal('heartbeat'),
    time: Timestamp.FromNumber,
  }),
)
type HeartbeatData = t.TypeOf<typeof HeartbeatData>

const MetaData = t.readonly(
  t.type({
    type: t.literal('meta'),
    message: t.string,
  }),
)
type MetaData = t.TypeOf<typeof MetaData>

const RunStatsData = t.readonly(
  t.type({
    type: t.literal('runStats'),
    counters: DurationMap,
    durations: CounterMap,
    gauges: GaugeMap,
  }),
)
type RunStatsData = t.TypeOf<typeof RunStatsData>

export class WebsocketCommandInterface implements CommandInterface {
  constructor(private readonly multiplexer: MultiplexedWebsocket, readonly sourceId: NodeId) {}

  alert: AlertRequest = (message: string, time: Timestamp) =>
    this.multiplexer
      .request(
        RequestTypes.Commands,
        AlertData.encode({
          type: 'alert',
          message,
          time,
        }),
      )
      .map(validateOrThrow(t.null))
      .map(_ => undefined)
      .toPromise()

  heartbeat: HeartbeatRequest = () =>
    this.multiplexer
      .request(
        RequestTypes.Commands,
        HeartbeatData.encode({
          type: 'heartbeat',
          time: Timestamp.now(),
        }),
      )
      .map(validateOrThrow(t.null))
      .map(_ => undefined)
      .toPromise()

  logging: LoggingRequest = (level: string, tag: string, message: string) => {
    if (tag === 'ws' && level !== 'ERROR') {
      return Promise.resolve()
    }
    return this.multiplexer
      .request(RequestTypes.Logging, { level, tag, message })
      .map(validateOrThrow(t.null))
      .map(_ => undefined)
      .toPromise()
  }

  meta: MetaRequest = (message: string) =>
    this.multiplexer
      .request(
        RequestTypes.Commands,
        MetaData.encode({
          type: 'meta',
          message,
        }),
      )
      .map(validateOrThrow(t.null))
      .map(_ => undefined)
      .toPromise()

  runStats: RunStatsRequest = (counters: DurationMap, durations: CounterMap, gauges: GaugeMap) =>
    this.multiplexer
      .request(
        RequestTypes.Commands,
        RunStatsData.encode({
          type: 'runStats',
          counters,
          durations,
          gauges,
        }),
      )
      .map(validateOrThrow(t.null))
      .map(_ => undefined)
      .toPromise()

  subscribe: SubscribeRequest = () =>
    this.multiplexer
      .request(
        RequestTypes.Commands,
        Subscribe.encode({
          type: 'subscribe',
        }),
      )
      .filter(command => ControlCommand.decode(command).isRight())
      .map(command => command as ControlCommand)
}
