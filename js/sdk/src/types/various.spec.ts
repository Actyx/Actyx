/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { gt, lt } from 'fp-ts/lib/Ord'
import { Milliseconds, NodeId, Timestamp, toMetadata, ActyxEvent } from '.'

describe('SourceId.random', () => {
  it('must create a random SourceID', () => expect(NodeId.random(42)).toHaveLength(42))
})

describe('ActyxEvent', () => {
  it('compares correctly', () => {
    const mkME = (eventId: string): ActyxEvent => ({ meta: { eventId } } as ActyxEvent)
    expect(lt(ActyxEvent.ord)(mkME('a'), mkME('b'))).toBe(true)
    expect(gt(ActyxEvent.ord)(mkME('a'), mkME('b'))).toBe(false)
    expect(lt(ActyxEvent.ord)(mkME('b'), mkME('a'))).toBe(false)
    expect(gt(ActyxEvent.ord)(mkME('b'), mkME('a'))).toBe(true)
  })
})

describe('Timestamp', () => {
  const now = 1545056028065

  it('Timestamp.now()', () => expect(Timestamp.now(now)).toEqual(1545056028065000))

  it('Timestamp.toSeconds()', () => expect(Timestamp.toSeconds(Timestamp.of(3 * 1e6))).toEqual(3))

  it('Timestamp.toMilliseconds()', () =>
    expect(Timestamp.toMilliseconds(Timestamp.of(3 * 1e6))).toEqual(3000))

  it('Timestamp.fromSeconds()', () => expect(Timestamp.fromSeconds(3)).toEqual(3 * 1e6))

  it('Timestamp.fromMilliseconds()', () => expect(Timestamp.fromMilliseconds(3)).toEqual(3 * 1e3))
})

describe('Milliseconds', () => {
  const now = 1545056028065
  it('Timestamp.fromAnyToMillis()', () => {
    const now0 = new Date().valueOf()
    expect(Milliseconds.fromAny(now0 * 1e3)).toEqual(now0)
    expect(Milliseconds.fromAny(now0)).toEqual(now0)
    expect(Milliseconds.fromAny(now)).toEqual(now)
  })
})

describe('toMetadata', () => {
  const ev = {
    offset: 5,
    stream: 'src',
    timestamp: 50_000,
    lamport: 12345,
    tags: ['tags'],
    payload: 'whatever',
    appId: 'xfoo',
  }

  const metadata = toMetadata('src')

  it('should generate eventId', () => {
    expect(metadata(ev).eventId).toEqual('0000000000012345/src')
    expect(metadata({ ...ev, lamport: Number.MAX_SAFE_INTEGER }).eventId).toEqual(
      '9007199254740991/src',
    )
  })

  it('should set isLocalEvent', () => {
    expect(metadata(ev).isLocalEvent).toBeTruthy()
    expect(metadata({ ...ev, stream: 'other' }).isLocalEvent).toBeFalsy()
  })
})
