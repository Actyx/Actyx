/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */

import { Event, _compareEvents } from './types'

describe('util', () => {
  it(`should correctly compare events`, () => {
    const e1: Event = {
      lamport: 1,
      psn: 10,
      sourceId: 'a',
    } as Event
    const e2: Event = {
      lamport: 2,
      psn: 20,
      sourceId: 'b',
    } as Event
    const mkL = (lamport: number): Event => ({ lamport } as Event)
    const mkS = (sourceId: string): Event => ({ ...mkL(0), sourceId } as Event)
    const mkO = (psn: number): Event => ({ ...mkL(0), ...mkS('a'), psn } as Event)
    expect(_compareEvents(mkL(1), mkL(2))).toBeLessThan(0)
    expect(_compareEvents(mkL(2), mkL(1))).toBeGreaterThan(0)
    expect(_compareEvents(mkS('a'), mkS('b'))).toBeLessThan(0)
    expect(_compareEvents(mkS('b'), mkS('a'))).toBeGreaterThan(0)
    expect(_compareEvents(mkO(1), mkO(2))).toBeLessThan(0)
    expect(_compareEvents(mkO(2), mkO(1))).toBeGreaterThan(0)
    expect(_compareEvents(e1, e2)).toBeLessThan(0)
  })
})
