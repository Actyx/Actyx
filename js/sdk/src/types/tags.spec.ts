/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Tag, Where } from '.'

type T0 = {
  type: '0'
  t0: object
}

type T1 = {
  type: '1'
  t1: object
}

const tag0 = Tag<T0>('0')
const tag1 = Tag<T1>('1')

type A = {
  type: 'A'
  data0: number
}

type B = {
  type: 'B'
  data1: string
}

type C = {
  type: 'C'
  data1: number
}

const tagA = Tag<A>('A')
const tagB = Tag<B>('B')

// Tag that covers 3 types
const abcTag = Tag<A | B | C>('ABC')

const tagWithQuotes = Tag<unknown>("a 'funny' tag")

// Satisfy TS (no unused var)
const ignoreUnusedVar = (_v: unknown) => undefined

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = tag0.and(tag1).or(tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const q1: Where<'hello??'> = q

    expect(q1.toWireFormat()).toMatchObject([{ tags: ['0', '1'] }, { tags: ['A'] }])
    expect(q1.toString()).toEqual("'0' & '1' | 'A'")
  })

  it('should insist on types?', () => {
    // @ts-expect-error
    const q2: Where<'A' | 'more-types'> = q

    // Must use q2 to avoid TS error...
    ignoreUnusedVar(q2)
  })

  it('should preserve the common event type', () => {
    // Overlap is 'A'
    const w = tagA.and(abcTag)

    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const w2: Where<never> = w

    expect(w2.toWireFormat()).toMatchObject([
      {
        tags: ['A', 'ABC'],
        local: false,
      },
    ])
    expect(w2.toString()).toEqual("'A' & 'ABC'")
  })

  it('should preserve local information', () => {
    // Overlap is 'A'
    const w = tagA.local().and(abcTag)

    expect(w.toWireFormat()).toMatchObject([
      {
        tags: ['A', 'ABC'],
        local: true,
      },
    ])
    expect(w.toString()).toEqual("'A' & 'ABC' & isLocal")
  })

  it('should union event types ', () => {
    const u = tagA.or(tagB)

    expect(u.toWireFormat()).toMatchObject([{ tags: ['A'] }, { tags: ['B'] }])
    expect(u.toString()).toEqual("'A' | 'B'")
  })

  it('should union event types (complex)', () => {
    // Surface now is 'A', 'B', and 'C'
    const u = tagA
      .local()
      .or(tagB.withId('some-id'))
      .or(abcTag)

    // Also covers 'C' now
    // @ts-expect-error
    const u2: Where<'A' | 'B'> = u

    expect(u.toWireFormat()).toMatchObject([
      { tags: ['A'] },
      {
        tags: ['B', 'B:some-id'],
        local: false,
      },
      {
        tags: ['ABC'],
      },
    ])
    expect(u.toString()).toEqual("'A' & isLocal | 'B' & 'B:some-id' | 'ABC'")
  })

  it('should OR several WHEREs', () => {
    const w0 = tag0.or(tag1)
    const w1 = tagA.or(tagB)

    const ww: Where<T0 | T1 | A | B> = w0.or(w1)

    expect(ww.toWireFormat()).toMatchObject([
      { tags: ['0'] },
      { tags: ['1'] },
      { tags: ['A'] },
      { tags: ['B'] },
    ])

    expect(ww.toString()).toEqual("'0' | '1' | 'A' | 'B'")
  })

  it('should tolerate tags with spaces and quotes', () => {
    const w0: Where<unknown> = tag0.or(tagWithQuotes)

    expect(w0.toWireFormat()).toMatchObject([{ tags: ['0'] }, { tags: ["a 'funny' tag"] }])

    expect(w0.toString()).toEqual("'0' | 'a ''funny'' tag'")
  })
})
