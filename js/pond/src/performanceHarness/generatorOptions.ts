/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
// TODO: change to commandpost, then `npm remove commander`
import * as commander from 'commander'
import * as moment from 'moment'
import { Timestamp } from '..'

const parseDuration = (text: string): number => moment.duration(text).asSeconds()
const parseDate = (text: string): Timestamp => Timestamp.of(moment(text).unix() * 1000)

const parser = commander
  .option(
    '-d, --duration <duration>',
    'Time period for which to generate events. E.g. P1D for one day, full format P1Y2M3DT4H5M6S',
    parseDuration,
  )
  .option(
    '-s, --start <date>',
    'ISO date to start generating events from. E.g. 2018-01-01T00:00:00',
    parseDate,
  )
  .option(
    '-p, --period <duration>',
    'Event generation period. E.g. PDT10s for one event every 10s',
    parseDuration,
  )
  .option('-r, --remote [url]', 'Address of a remote RDS backed to sync with')
  .option('-n, --db-name <db name>', 'Identifier for the database. May be used as a file name.')
  .option('-c, --clean', 'Shall the specified database be cleaned before running?')
  .option('-m, --mode', 'The persistence mode. May be transient, persistent or ipfs')

export type GeneratorOptions = Readonly<{
  /**
   * Start of event generation as a timestamp
   */
  start: Timestamp
  /**
   * Duration of event generation in seconds
   */
  duration: number
  /**
   * Event generation period in seconds
   */
  period: number
  /**
   * Optional remote to sync to
   */
  remote?: string
  /**
   * Optional db name
   */
  dbName?: string
  /**
   * Shall the specified database be cleaned before running?
   */
  clean: boolean
  /**
   * The persistence mode
   */
  mode: 'transient' | 'persistent' | 'ipfs'
}>

export const GeneratorOptions = {
  help: () => parser.help(),
  parse: (args: string[]): GeneratorOptions => {
    // any because type definition file is
    const parsed = parser.parse(args)
    return {
      start: parsed.start,
      duration: parsed.duration,
      period: parsed.period,
      remote: parsed.remote,
      dbName: parsed.dbName,
      clean: parsed.clean || false,
      mode: parsed.mode || 'ipfs',
    }
  },
}
