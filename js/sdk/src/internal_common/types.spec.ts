/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { Event, _compareEvents } from '.'
import { ConnectivityStatus } from './types'

describe('util', () => {
  it(`should correctly compare events`, () => {
    const e1: Event = {
      lamport: 1,
      offset: 10,
      stream: 'a',
    } as Event
    const e2: Event = {
      lamport: 2,
      offset: 20,
      stream: 'b',
    } as Event
    const mkL = (lamport: number): Event => ({ lamport } as Event)
    const mkS = (stream: string): Event => ({ ...mkL(0), stream } as Event)
    const mkO = (offset: number): Event => ({ ...mkL(0), ...mkS('a'), offset } as Event)
    expect(_compareEvents(mkL(1), mkL(2))).toBeLessThan(0)
    expect(_compareEvents(mkL(2), mkL(1))).toBeGreaterThan(0)
    expect(_compareEvents(mkS('a'), mkS('b'))).toBeLessThan(0)
    expect(_compareEvents(mkS('b'), mkS('a'))).toBeGreaterThan(0)
    expect(_compareEvents(mkO(1), mkO(2))).toBeLessThan(0)
    expect(_compareEvents(mkO(2), mkO(1))).toBeGreaterThan(0)
    expect(_compareEvents(e1, e2)).toBeLessThan(0)
  })
})

describe('connectivity status codes', () => {
  it('should decode FullyConnected', () => {
    const v = {
      status: 'FullyConnected',
      inCurrentStatusForMs: 100,
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode PartiallyConnected with empty specials', () => {
    const v = {
      status: 'PartiallyConnected',
      inCurrentStatusForMs: 100,
      swarmConnectivityLevel: 70,
      eventsToRead: 5,
      eventsToSend: 6,
      specialsDisconnected: [],
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode PartiallyConnected with filled specials', () => {
    const v = {
      status: 'PartiallyConnected',
      inCurrentStatusForMs: 100,
      swarmConnectivityLevel: 70,
      eventsToRead: 5,
      eventsToSend: 6,
      specialsDisconnected: ['some-source'],
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode NotConnected', () => {
    const v = {
      status: 'NotConnected',
      inCurrentStatusForMs: 2000000,
      eventsToRead: 5,
      eventsToSend: 6,
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })
})
