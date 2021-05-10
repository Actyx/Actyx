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
import * as debug from 'debug'
import * as util from 'util'
import { LogFunction, LoggersInternal, LogLeech, makeLogPattern, mkLogger } from '.'

describe('makeLogPattern', () => {
  it('must properly configure the browser', () => {
    expect(makeLogPattern(['db'])).toEqual('*,-db:((?!error).)*,*:error')
  })
})

describe('mkLogger', () => {
  // Thanks @ https://stackoverflow.com/questions/25245716/remove-all-ansi-colors-styles-from-strings
  // eslint-disable-next-line no-control-regex
  const allColorCodes = /[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g

  const globalLeechBefore = LoggersInternal.globalLogLeech

  beforeAll(() => {
    debug.enable('foo')
  })

  afterAll(() => {
    debug.enable('-foo')
  })

  afterEach(() => {
    while (outputCapture.length > 0) {
      outputCapture.pop()
    }
    LoggersInternal.globalLogLeech = globalLeechBefore
  })

  const outputCapture: string[] = []
  const logToCapture: LogFunction = (...args) => outputCapture.push(util.format(...args)) // util.format is what debug also calls.
  const leechToCapture: LogLeech = (_nspace, args) => outputCapture.push(args)

  const assertEntry = (expected: string = 'hello log', output: string[] = outputCapture) => {
    expect(output.length).toEqual(1)
    expect(output[0].replace(allColorCodes, '')).toContain(expected)
  }

  it('should allow changing log function on logger creation', () => {
    const l = mkLogger('foo', logToCapture)
    l('hello log')
    assertEntry('foo hello log')
  })

  it('should forward invocation args to globalLogLeech', () => {
    LoggersInternal.globalLogLeech = leechToCapture
    const l = mkLogger('foo')

    l('hello log')
    assertEntry()
  })

  it('should catch errors thrown by global log leech and report them to normal log output', () => {
    LoggersInternal.globalLogLeech = (_arg0, _arg1) => {
      throw new Error('Whoops')
    }
    const l = mkLogger('foo', logToCapture)

    l('hello log')

    expect(outputCapture.length).toEqual(2)
    expect(outputCapture[0].replace(allColorCodes, '')).toContain('hello log')

    const errorLogLine = outputCapture[1].replace(allColorCodes, '')
    expect(errorLogLine).toContain('Error while leeching log message:')
    expect(errorLogLine).toContain('Whoops')
    expect(errorLogLine).toContain('logging.ts:') // stacktrace
  })

  it('should call both globalLogLeech and original log function', () => {
    LoggersInternal.globalLogLeech = leechToCapture
    const normalOutput: string[] = []
    const l = mkLogger('foo', args => normalOutput.push(args))

    l('hello log')
    assertEntry()
    assertEntry('foo hello log', normalOutput)
  })
})
