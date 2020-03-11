/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { mapOption } from 'fp-ts/lib/Array'
import { fromNullable, none, Option, some } from 'fp-ts/lib/Option'
import * as t from 'io-ts'
import { createOptionFromNullable } from 'io-ts-types'
import { PathReporter } from 'io-ts/lib/PathReporter'
import { groupBy } from 'ramda'
import { Observable } from 'rxjs'
import { log } from '../loggers'
import { CommandValidator, FishName, FishType, SendCommand, Timestamp } from '../types'
import { unreachableOrElse } from '../util'
import mkWebSocket from './websocket'

export const enum MessageType {
  Pond = 'pond',
  NfcReading = 'nfcreading',
  DataMatrixCode = 'datamatrixcode',
}

export const PondMessage = t.type({
  type: t.literal('pond'),
  semantics: t.string,
  name: createOptionFromNullable(FishName.FromString),
  command: t.unknown,
})
export type PondMessage = t.TypeOf<typeof PondMessage>

export const NfcReadingMessage = t.type({
  type: t.literal('nfcreading'),
  uid: t.string,
  /**
   * The timestamp is needed, as there could be quite some delay between the reading
   * and the received command within this fish.
   */
  timestamp: Timestamp.FromNumber,
})
export type NfcReadingMessage = t.TypeOf<typeof NfcReadingMessage>

export const DataMatrixCodeMessage = t.type({
  type: t.literal('datamatrixcode'),
  text: t.string,
  timestamp: Timestamp.FromNumber,
})
export type DataMatrixCodeMessage = t.TypeOf<typeof DataMatrixCodeMessage>

const BatteryStateMessage = t.type({
  type: t.literal('batterystate'),
  level: t.number,
})

export const WebSocketMessage = t.taggedUnion('type', [
  PondMessage,
  NfcReadingMessage,
  DataMatrixCodeMessage,
  BatteryStateMessage,
])
export type WebSocketMessage = t.TypeOf<typeof WebSocketMessage>

export type WebSocketPondSubscription<C> = Readonly<{
  type: MessageType.Pond
  semantics: string
  fish: FishType<C, any, any>
  commandValidator: CommandValidator<C>
  fishName?: FishName
}>

export type WebSocketNfcReadingSubscription<C> = Readonly<{
  type: MessageType.NfcReading
  fish: FishType<C, any, any>
  mapToCommand: (msg: NfcReadingMessage) => C
  fishName: FishName
}>

export type WebSocketDataMatrixCodeSubscription<C> = Readonly<{
  type: MessageType.DataMatrixCode
  fish: FishType<C, any, any>
  mapToCommand: (msg: DataMatrixCodeMessage) => C
  fishName: FishName
}>

export type WebSocketSubscription<C> =
  | WebSocketPondSubscription<C>
  | WebSocketNfcReadingSubscription<C>
  | WebSocketDataMatrixCodeSubscription<C>

export type WebSocketSubscriptions = ReadonlyArray<WebSocketSubscription<any>>

export const WebSocketSubscription = {
  ofPond: <C>(
    semantics: string,
    fish: FishType<C, any, any>,
    commandValidator: CommandValidator<C>,
    fishName?: FishName,
  ): WebSocketPondSubscription<C> => ({
    type: MessageType.Pond,
    semantics,
    fish,
    commandValidator,
    fishName,
  }),
  /**
   * @deprecated
   */
  ofNfcReading: <C>(
    fish: FishType<C, any, any>,
    mapToCommand: (msg: NfcReadingMessage) => C,
    fishName: FishName,
  ): WebSocketNfcReadingSubscription<C> => ({
    type: MessageType.NfcReading,
    fish,
    mapToCommand,
    fishName,
  }),
  ofDataMatrixCode: <C>(
    fish: FishType<C, any, any>,
    mapToCommand: (msg: DataMatrixCodeMessage) => C,
    fishName: FishName,
  ): WebSocketDataMatrixCodeSubscription<C> => ({
    type: MessageType.DataMatrixCode,
    fish,
    mapToCommand,
    fishName,
  }),
}

export type Protocol = 'ws' | 'wss'
export type Credentials = Readonly<{ user: string; password: string }>

export type WsConfig = Readonly<{
  host: string
  protocol: Protocol
  credentials?: Credentials
  wsSubscriptions: WebSocketSubscriptions
}>

const getFishCommandFromPondMessage = ({ name, command }: PondMessage) => (
  entry: WebSocketPondSubscription<any>,
): Option<SendCommand<any>> => {
  return name.orElse(() => fromNullable(entry.fishName)).chain(fishName =>
    entry.commandValidator(command).fold(
      error => {
        log.ws.error('Received invalid command! Error: `%s`. Command: %j', error, command)
        return none
      },
      cmd => some(SendCommand.of(entry.fish, fishName, cmd)),
    ),
  )
}

const toFishCommands = (subs: WebSocketSubscriptions) => {
  const nfcSubs = subs.filter(
    (x): x is WebSocketNfcReadingSubscription<any> => x.type === MessageType.NfcReading,
  )
  const dmcSubs = subs.filter(
    (x): x is WebSocketDataMatrixCodeSubscription<any> => x.type === MessageType.DataMatrixCode,
  )
  const pondSubs = groupBy(
    x => x.semantics,
    subs.filter((x): x is WebSocketPondSubscription<any> => x.type === MessageType.Pond),
  )

  return (message: WebSocketMessage): ReadonlyArray<SendCommand<any>> => {
    const result = WebSocketMessage.decode(message)
    return result
      .mapLeft(() => {
        log.ws.error('Failed to decode command', PathReporter.report(result))
      })
      .map(msg => {
        switch (msg.type) {
          case 'pond': {
            return mapOption(pondSubs[msg.semantics] || [], getFishCommandFromPondMessage(msg))
          }
          case 'nfcreading': {
            return nfcSubs.map(({ fish, fishName, mapToCommand }) =>
              SendCommand.of(fish, fishName, mapToCommand(msg)),
            )
          }
          case 'datamatrixcode': {
            return dmcSubs.map(({ fish, fishName, mapToCommand }) =>
              SendCommand.of(fish, fishName, mapToCommand(msg)),
            )
          }
          case 'batterystate': {
            return []
          }
          default: {
            return unreachableOrElse(msg, [])
          }
        }
      })
      .getOrElse([])
  }
}

const mkWsServerUrl = ({ host, protocol, credentials }: WsConfig) =>
  `${protocol}://${credentials ? `${credentials.user}:${credentials.password}@` : ''}${host}`

export const processWebSocketMessage = (
  stream: Observable<WebSocketMessage>,
  subs: WebSocketSubscriptions,
): Observable<SendCommand<any>> =>
  stream.do(m => log.ws.debug('Received WebSocket Message: ', m)).concatMap(toFishCommands(subs))

export const enableWsFeeder = (config: WsConfig): Observable<SendCommand<any>> => {
  const { incoming } = mkWebSocket<WebSocketMessage>({ url: mkWsServerUrl(config) })

  return processWebSocketMessage(incoming, config.wsSubscriptions)
}
