/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable, ReplaySubject } from 'rxjs'
import { NodeId, Timestamp } from '@actyx/sdk'
import { CounterMap, DurationMap, GaugeMap } from '../util'
import {
  AlertRequest,
  CommandInterface,
  HeartbeatRequest,
  LoggingRequest,
  MetaRequest,
  RunStatsRequest,
  SubscribeRequest,
} from './commandInterface'

type LoggingEventDataType = {
  message: string
  level: string
  tag: string
}
type AlertsDataType = {
  message: string
  time: Timestamp
}
type RunStatsDataType = {
  counters: DurationMap
  durations: CounterMap
  gauges: GaugeMap
}
export const mockCommandInterface: () => CommandInterface = () => {
  const sourceId = NodeId.of('MOCK')
  const alertSubject = new ReplaySubject<AlertsDataType>(1e3)
  const heartbeatSubject = new ReplaySubject<Timestamp>(1e3)
  const logSubject = new ReplaySubject<LoggingEventDataType>(1e3)
  const metaSubject = new ReplaySubject<string>(1e3)
  const runStatsSubject = new ReplaySubject<RunStatsDataType>(1e3)

  let subscriptions = 0

  const alert: AlertRequest = (message, time) => {
    alertSubject.next({ message, time })
    return Promise.resolve()
  }
  const heartbeat: HeartbeatRequest = () => {
    heartbeatSubject.next(Timestamp.now())
    return Promise.resolve()
  }
  const logging: LoggingRequest = (level, tag, message) => {
    logSubject.next({ level, tag, message })
    return Promise.resolve()
  }
  const meta: MetaRequest = message => {
    metaSubject.next(message)
    return Promise.resolve()
  }
  const runStats: RunStatsRequest = (counters, durations, gauges) => {
    runStatsSubject.next({ counters, durations, gauges })
    return Promise.resolve()
  }
  const subscribe: SubscribeRequest = () => {
    subscriptions++
    return Observable.never()
  }

  return {
    sourceId,
    alert,
    heartbeat,
    logging,
    meta,
    runStats,
    subscribe,

    alertSubject,
    heartbeatSubject,
    logSubject,
    metaSubject,
    runStatsSubject,
    subscriptions,
  }
}
