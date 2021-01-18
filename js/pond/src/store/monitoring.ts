/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
// #region impl

import * as debug from 'debug'
import { Observable } from 'rxjs'
import { format } from 'util'
import { Timestamp } from '../types'
import {
  CommandInterface,
  ControlCommand,
  ControlCommandType,
  LogLevel,
  RunStatsRequest,
} from '../commandInterface'
import { getMemoryUsage, isNode, Loggers, LoggersInternal, noop } from '../util'
import { runStats } from '../util/runStats'
import { log } from './loggers'

// eslint-disable-next-line @typescript-eslint/ban-ts-ignore
// @ts-ignored

const serialNumber = '<unknown>'

const sendMeta = (commandInterface: CommandInterface) => (message: string) =>
  commandInterface.meta(message)

export const prittifyArgs = (msg: any | undefined, args: ReadonlyArray<any>) => {
  const traceObject = (val: any, lng: number) => {
    if (typeof val !== 'object') {
      return val
    }
    const objectOutput = JSON.stringify(val)
      .substr(0, lng * 1.2)
      // eslint-disable-next-line no-useless-escape
      .replace(/\"/g, '')
    return objectOutput.length > lng ? objectOutput.substr(0, lng) + '...' : objectOutput
  }
  const embeddedArgs =
    typeof msg === 'string'
      ? msg
          .split(/(%d|%j|%o|%s)/)
          .filter(x => x)
          .filter(x => x[0] === '%').length
      : 0
  return args.map((val, idx) => (idx >= embeddedArgs ? traceObject(val, 200) : val))
}

const dissectAndSend = (commandInterface: CommandInterface) => {
  return function customLog(namespace: string, msg?: any, ...args: any[]): void {
    const [tag, l] = namespace.split(':', 2)
    const level = LogLevel.of(l as keyof Loggers)
    const message = format.call(null, msg, ...prittifyArgs(msg, args))
    // Serialnumber is removed. final logmessage will be defined from the SRE-team

    commandInterface.logging(level, tag, message).catch(console.error)
  }
}
const sendDistressCall = (commandInterface: CommandInterface) => (msg?: any, ...args: any[]) =>
  commandInterface.alert(format.call(null, msg, ...prittifyArgs(msg, args)), Timestamp.now())

export const enableLogging = (commandInterface: CommandInterface, namespaces?: string) => {
  // TODO: set localStorage.debug to persist it?
  namespaces && debug.enable(namespaces)
  LoggersInternal.globalLogLeech = dissectAndSend(commandInterface)
}
const disableLogging = () => (debug as any).disable()

const handleControlCommand = (commandInterface: CommandInterface) => (
  command: ControlCommand,
): void => {
  const source = commandInterface.sourceId
  /**
   * Send a command to stdout as well as to the monitoring topic as a meta message
   *
   * Note: this logs directly to console to avoid a log loop. We must not log through
   * the debug logging framework here since we intercept those calls.
   *
   * @param msg the message
   */
  const sendMetaAndLog = (msg: string) => {
    console.info(msg)

    sendMeta(commandInterface)(msg)
  }

  switch (command.type) {
    case ControlCommandType.SetDebug: {
      const { value, sources, target } = command
      const apply = sources && (sources.includes('all') || sources.includes(source))

      if (apply) {
        switch (target) {
          case undefined: {
            const enabled = value !== ''
            if (enabled) {
              enableLogging(commandInterface, value)
              sendMetaAndLog(`enabling logging using pattern ${value}`)
            } else {
              disableLogging()
              sendMetaAndLog('disabling logging')
            }
            break
          }

          case 'shell': {
            if (!isNode) {
              sendMetaAndLog('setting log level in shell app')
              fetch('http://localhost:8080/logLevel', {
                mode: 'no-cors',
                method: 'post',
                body: value,
              })
                .then(() => sendMetaAndLog('log level set in shell app'))
                .catch(err => sendMetaAndLog(`failed to set log level in shell app: ${err}`))
            }
            break
          }
        }
      }

      break
    }

    case ControlCommandType.Backup: {
      // TODO: probably restart app after restore? (or in shell app)
      break
    }

    default: {
      // just ignore the command, it is probably handled by the store or shell-app
      break
    }
  }
}

const sendRunStats = (runStatsRequest: RunStatsRequest): void => {
  Object.entries(getMemoryUsage()).map(([key, value]) =>
    runStats.gauges.set('memory.' + key, value),
  )

  const counters = runStats.counters.current()
  const durations = runStats.durations.getAndClear()
  const gauges = runStats.gauges.current()

  log.stats.debug('%j %j %j', counters, durations, gauges)

  runStatsRequest(durations, counters, gauges)
}

const mkMonitoring = (
  commandInterface: CommandInterface,
  runStatsPeriodMs: number = 30000,
  heartbeatMs: number = 30000,
): Monitoring => {
  enableLogging(commandInterface)

  const commands = commandInterface.subscribe().do(handleControlCommand(commandInterface))

  const heartbeat = Observable.timer(heartbeatMs, heartbeatMs).do(() =>
    commandInterface.heartbeat(),
  )
  const statsLoop = Observable.timer(runStatsPeriodMs, runStatsPeriodMs).do(() => {
    sendRunStats(commandInterface.runStats)
  })

  const sub = Observable.merge(heartbeat, commands, statsLoop).subscribe({
    error: e => {
      const msg = 'Fatal error. terminating pond loops!'
      log.monitoring.debug(msg, e)

      sendDistressCall(commandInterface)(msg, e)
    },
  })

  return {
    meta: sendMeta(commandInterface),
    sendMessage: dissectAndSend(commandInterface),
    distress: sendDistressCall(commandInterface),
    dispose: () => sub.unsubscribe(),
  }
}
// #endregion

export interface Monitoring {
  meta: (message: string) => void
  sendMessage: (message?: any, ...args: any[]) => void
  distress: (message: string, ...args: any[]) => void
  dispose: () => void
}

const mockMonitoring: Monitoring = {
  meta: noop,
  sendMessage: noop,
  distress: noop,
  dispose: noop,
}

export const Monitoring = {
  of: mkMonitoring,
  mock: () => mockMonitoring,
}
