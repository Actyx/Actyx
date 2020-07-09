/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  AcknowledgeAlarm,
  AlarmEvent,
  alarmFishType as AlarmFish,
  Command,
  Event,
  project,
  SequenceNumber as sn,
  State,
} from './alarm.support.test'
import { FishTestFunctions } from './testkit'
import {
  Envelope,
  FishName,
  Lamport,
  Source,
  SourceId as sid,
  SourceId,
  Timestamp as ts,
} from './types'

const alarm = FishTestFunctions.of<Command, Event, State>(AlarmFish, {})

describe('The Alarm Fish', () => {
  const sourceId: SourceId = sid.of('dummyId')
  const source: Source = Source.of(AlarmFish, FishName.of('filling'), sourceId)
  const message: string = 'hello world'
  const reg1: Command = { type: 'registerAlarm', timestamp: ts.of(12), source, message }
  const evt1: (n: number) => AlarmEvent = (n: number) => ({
    type: 'alarmRaised',
    sequence: sn.of(n),
    timestamp: ts.of(12),
    source,
    message,
  })
  const evt1Event: (n: number) => Envelope<AlarmEvent> = (n: number) => ({
    source: Source.of(AlarmFish, FishName.of('default'), sourceId),
    sequence: sn.of(n),
    timestamp: ts.of(1235),
    lamport: Lamport.of(1235),
    payload: evt1(n),
  })
  const ack1: (n: number) => AcknowledgeAlarm = (n: number) => ({
    type: 'acknowledgeAlarm',
    sequence: sn.of(n),
  })
  const ack1evt: (n: number) => Event = (n: number) => ({
    type: 'alarmAcknowledged',
    sequence: sn.of(n),
  })
  const ack1Event: (n: number) => Envelope<Event> = (n: number) => ({
    source: Source.of(AlarmFish, FishName.of('default'), sourceId),
    sequence: sn.of(n),
    timestamp: ts.of(1235),
    lamport: Lamport.of(1235),
    payload: ack1evt(n),
  })

  const initial = alarm.initialState(FishName.of('default'), sourceId)
  const registered = alarm.onEvent(initial, evt1Event(0))
  const registered1 = alarm.onEvent(initial, evt1Event(1))
  const registered2 = alarm.onEvent(registered, evt1Event(1))

  it('must accept registration commands', () => {
    expect(AlarmFish.onCommand(initial, reg1)).toEqual([evt1(0)])
    expect(AlarmFish.onCommand(registered, reg1)).toEqual([evt1(1)])
  })

  it('must accept acknowledgement commands', () => {
    expect(AlarmFish.onCommand(initial, ack1(0))).toEqual([])
    expect(AlarmFish.onCommand(registered, ack1(0))).toEqual([ack1evt(0)])
    expect(AlarmFish.onCommand(registered, ack1(1))).toEqual([])
    expect(AlarmFish.onCommand(registered2, ack1(0))).toEqual([ack1evt(0)])
    expect(AlarmFish.onCommand(registered2, ack1(1))).toEqual([ack1evt(1)])
  })

  it('must acknowledge alarms', () => {
    expect(project(alarm.onEvent(initial, ack1Event(0)))).toEqual(project(initial))
    expect(project(alarm.onEvent(registered, ack1Event(0)))).toEqual(project(initial))
    expect(project(alarm.onEvent(registered2, ack1Event(0)))).toEqual(project(registered1))
    expect(project(alarm.onEvent(registered2, ack1Event(1)))).toEqual(project(registered))
  })

  it('must show the right alarms', () => {
    expect(project(initial)).toEqual({ open: [] })
    expect(project(registered)).toEqual({ open: [evt1(0)] })
    expect(project(registered1)).toEqual({ open: [evt1(1)] })
    expect(project(registered2)).toEqual({ open: [evt1(0), evt1(1)] })
  })

  it('must tolerate unknown events', () => {
    const ev: any = { source, sequence: sn.zero, timestamp: ts.zero, payload: 'hello' }
    expect(alarm.onEvent(initial, ev)).toEqual(initial)
  })

  it('must tolerate unknown commands', () => {
    expect(AlarmFish.onCommand(initial, 'buh' as any)).toEqual([])
  })
})
