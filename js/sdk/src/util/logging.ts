/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import * as debug from 'debug'

/**
 * Generic logging function signature.
 * @public
 */
export type LogFunction = ((first: any, ...rest: any[]) => void)

/**
 * A concrete logger that has a namespace and a flag indicating whether
 * it’s enabled or logged messages will just be silently swallowed.
 */
export interface Logger extends LogFunction {
  // Can never be changed after initialization
  readonly namespace: string
  // Changed via debug.enable(namespaces) call
  readonly enabled: boolean
}

/**
 * A collection of loggers of different severity for a fixed topic.
 */
export type Loggers = {
  error: Logger
  warn: Logger
  debug: Logger
  info: Logger
}

/**
 * Loggers which just buffer messages.
 */
export type TestLoggers = {
  errors: ReadonlyArray<string>
  warnings: ReadonlyArray<string>
  error: Logger
  warn: Logger
  debug: Logger
  info: Logger
}
function mkTestLogger(dump: string[]): Logger {
  function logger(...args: any[]): void {
    dump.push(args.map(x => JSON.stringify(x)).join(':'))
  }
  logger.namespace = 'test'
  logger.enabled = true

  return logger
}
export const mkTestLoggers = (): TestLoggers => {
  const errors: string[] = []
  const warnings: string[] = []

  return {
    errors,
    warnings,
    error: mkTestLogger(errors),
    warn: mkTestLogger(warnings),
    info: mkTestLogger([]),
    debug: mkTestLogger([]),
  }
}

// The goal is to make our logger look exactly like one from the 'debug' library,
// only we potentially leech the inputs - before they are formatted!
export const mkLogger = (topic: string, logFnOverride?: LogFunction) => {
  const actualLogger = debug(topic)

  if (logFnOverride) {
    actualLogger.log = logFnOverride
  }

  const logger: LogFunction = (first: any, ...rest: any[]) => {
    if (actualLogger.enabled) {
      actualLogger(first, ...rest)
      try {
        LoggersInternal.globalLogLeech(actualLogger.namespace, first, ...rest)
      } catch (e) {
        actualLogger('Error while leeching log message: ', e)
      }
    }
  }

  // Easiest way to supply the readonly namespace/enabled properties required by the interface.
  Object.setPrototypeOf(logger, actualLogger)

  return logger as Logger
}

// todo: special treatment for errors?
export const mkLoggers: (topic: string) => Loggers = topic => ({
  error: mkLogger(`${topic}:error`), // Options description available in README
  warn: mkLogger(`${topic}:warn`),
  info: mkLogger(`${topic}:info`),
  debug: mkLogger(`${topic}:debug`),
})

export type LogLeech = (namespace: string, first: any, ...rest: any[]) => void
export const globalLogLeech: LogLeech = () => {
  /* Nothing by default. Overridden by monitoring module. */
  /**
   * If you want to add another global log consumer,
   * consider extending the API here to hold any number
   * of consumers that are each called for every log invocation...
   * like more extensive logging frameworks let you. */
}

export const LoggersInternal = {
  globalLogLeech,
  testLoggers: mkTestLoggers,
}

/** Loggers associated methods. @public */
export const Loggers = {
  of: mkLoggers,
}

/**
 * Build logging pattern for consumption by the `debug` library.
 * @public
 */
export const makeLogPattern = (excludeModules: string[]) =>
  `*,${excludeModules.map(x => `-${x}:((?!error).)*`).join(',')},*:error`

/**
 * Utility function to enable all logging with exception for passed in logger namespaces.
 * For excluded logger namespaces errors will still be logged!
 * @public
 */
export const enableAllLoggersExcept = (excludeModules: string[]): void => {
  // $ExpectError
  localStorage.debug = makeLogPattern(excludeModules)
}
