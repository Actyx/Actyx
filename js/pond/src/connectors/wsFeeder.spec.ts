/* eslint-disable @typescript-eslint/no-explicit-any */

import * as t from 'io-ts'
import { Server } from 'mock-socket'
import { Observable } from 'rxjs'
import {
  CommandValidator,
  FishName,
  FishType,
  OnStateChange,
  Semantics,
  Target,
  Timestamp,
  ValidationFailure,
} from '../types'
import {
  DataMatrixCodeMessage,
  enableWsFeeder,
  NfcReadingMessage,
  processWebSocketMessage,
  WebSocketSubscription,
  WebSocketSubscriptions,
  WsConfig,
} from './wsFeeder'

export const NfcReading = t.type({
  type: t.literal('nfcReading'),
  id: t.string,
})
export type NfcReading = t.TypeOf<typeof NfcReading>

export const DataMatrixCode = t.type({
  type: t.literal('datamatrixcode'),
  text: t.string,
})
export type DataMatrixCode = t.TypeOf<typeof DataMatrixCode>

export const CommandIO = t.union([NfcReading, DataMatrixCode])
export type TestCommand = t.TypeOf<typeof CommandIO>
type TestEvents = TestCommand

const TestCommand = {
  nfcReading: (id: string): TestCommand => ({ type: 'nfcReading', id }),
  dataMatrixCode: (text: string): TestCommand => ({ type: 'datamatrixcode', text }),
}

export const commandValidator: CommandValidator<TestCommand> = x =>
  CommandIO.decode(x).mapLeft(() => ValidationFailure.InvalidPayload)

type TestState = {}

const TestFish: FishType<TestCommand, TestEvents, TestState> = FishType.of({
  semantics: Semantics.of('ws-test'),
  initialState: () => ({ state: {} }),
  onStateChange: OnStateChange.publishPrivateState(),
})

const fishName = FishName.of('foo')
const fishName2 = FishName.of('bar')
const wsSubscriptions: WebSocketSubscriptions = [
  WebSocketSubscription.ofNfcReading(TestFish, ({ uid }) => TestCommand.nfcReading(uid), fishName),
  WebSocketSubscription.ofDataMatrixCode(
    TestFish,
    ({ text }) => TestCommand.dataMatrixCode(text),
    fishName,
  ),
  WebSocketSubscription.ofPond('whatever', TestFish, commandValidator, fishName),
]

const dataMatrixCodeMessage: DataMatrixCodeMessage = {
  type: 'datamatrixcode',
  text: 'some code',
  timestamp: Timestamp.of(42),
}

const nfcReadingMessage: NfcReadingMessage = {
  type: 'nfcreading',
  uid: 'some id',
  timestamp: Timestamp.of(42),
}

export const pondNfcReadingMessage: any = {
  type: 'pond',
  semantics: 'whatever',
  command: TestCommand.nfcReading('42'),
}
export const pondDataMatrixCodeMessage: any = {
  type: 'pond',
  semantics: 'whatever',
  command: TestCommand.dataMatrixCode('code gray'),
}

describe('processWebSocketMessage', () => {
  it('should handle NfcReadingMessage', () =>
    expect(
      processWebSocketMessage(Observable.of(nfcReadingMessage), wsSubscriptions).toPromise(),
    ).resolves.toEqual({
      command: { id: 'some id', type: 'nfcReading' },
      target: Target.of(TestFish, fishName),
    }))

  it('should handle DataMatrixCodeMessage', () =>
    expect(
      processWebSocketMessage(Observable.of(dataMatrixCodeMessage), wsSubscriptions).toPromise(),
    ).resolves.toEqual({
      command: { text: 'some code', type: 'datamatrixcode' },
      target: Target.of(TestFish, fishName),
    }))

  it('should handle pondNfcReadingMessage', () =>
    expect(
      processWebSocketMessage(Observable.of(pondNfcReadingMessage), wsSubscriptions).toPromise(),
    ).resolves.toEqual({
      command: { id: '42', type: 'nfcReading' },
      target: Target.of(TestFish, fishName),
    }))

  it('should handle dataMatrixCodeMessage', () =>
    expect(
      processWebSocketMessage(Observable.of(dataMatrixCodeMessage), wsSubscriptions).toPromise(),
    ).resolves.toEqual({
      command: { text: 'some code', type: 'datamatrixcode' },
      target: Target.of(TestFish, fishName),
    }))

  it('should handle PondMessage, NfcReadingMessage dataMatrixCodeMessage and pondDataMatrixCodeMessage', () =>
    expect(
      processWebSocketMessage(
        Observable.from([
          nfcReadingMessage,
          pondNfcReadingMessage,
          dataMatrixCodeMessage,
          pondDataMatrixCodeMessage,
        ]),
        wsSubscriptions,
      )
        .toArray()
        .toPromise(),
    ).resolves.toEqual([
      {
        command: { id: 'some id', type: 'nfcReading' },
        target: Target.of(TestFish, fishName),
      },
      {
        command: { id: '42', type: 'nfcReading' },
        target: Target.of(TestFish, fishName),
      },
      {
        command: { text: 'some code', type: 'datamatrixcode' },
        target: Target.of(TestFish, fishName),
      },
      {
        command: { text: 'code gray', type: 'datamatrixcode' },
        target: Target.of(TestFish, fishName),
      },
    ]))

  it('should handle any pondNfcReadingMessage when many subscriptions for one semantics', () =>
    expect(
      processWebSocketMessage(Observable.of(pondNfcReadingMessage), [
        WebSocketSubscription.ofPond('whatever', TestFish, commandValidator, fishName),
        WebSocketSubscription.ofPond('whatever', TestFish, commandValidator, fishName2),
      ])
        .toArray()
        .toPromise(),
    ).resolves.toEqual([
      {
        command: { id: '42', type: 'nfcReading' },
        target: Target.of(TestFish, fishName),
      },
      {
        command: { id: '42', type: 'nfcReading' },
        target: Target.of(TestFish, fishName2),
      },
    ]))

  it('should handle pondNfcReadingMessage and fish name from the message should take precedence', () =>
    expect(
      processWebSocketMessage(
        Observable.of({ ...pondNfcReadingMessage, name: 'name from message' }),
        [WebSocketSubscription.ofPond('whatever', TestFish, commandValidator, fishName)],
      ).toPromise(),
    ).resolves.toEqual({
      command: { id: '42', type: 'nfcReading' },
      target: Target.of(TestFish, FishName.of('name from message')),
    }))
})

describe('enableWsFeeder', () => {
  it('should handle ws messages', () => {
    const config: WsConfig = {
      host: 'localhost:6666/ws',
      protocol: 'ws',
      wsSubscriptions,
    }
    const srver = new Server(`${config.protocol}://${config.host}`)
    srver.on('connection', (socket: any) => {
      socket.send(JSON.stringify(nfcReadingMessage))
    })
    return expect(
      enableWsFeeder(config)
        .first()
        .toPromise(),
    ).resolves.toEqual({
      command: { id: 'some id', type: 'nfcReading' },
      target: Target.of(TestFish, fishName),
    })
  })
})
