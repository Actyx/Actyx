/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
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

    expect(q1.toV1WireFormat()).toMatchObject([{ tags: ['0', '1'] }, { tags: ['A'] }])
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

    expect(w2.toV1WireFormat()).toMatchObject([
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

    expect(w.toV1WireFormat()).toMatchObject([
      {
        tags: ['A', 'ABC'],
        local: true,
      },
    ])
    expect(w.toString()).toEqual("'A' & 'ABC' & isLocal")
  })

  it('should union event types ', () => {
    const u = tagA.or(tagB)

    expect(u.toV1WireFormat()).toMatchObject([{ tags: ['A'] }, { tags: ['B'] }])
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

    expect(u.toV1WireFormat()).toMatchObject([
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

    expect(ww.toV1WireFormat()).toMatchObject([
      { tags: ['0'] },
      { tags: ['1'] },
      { tags: ['A'] },
      { tags: ['B'] },
    ])

    expect(ww.toString()).toEqual("'0' | '1' | 'A' | 'B'")
  })

  it('should tolerate tags with spaces and quotes', () => {
    const w0: Where<unknown> = tag0.or(tagWithQuotes)

    expect(w0.toV1WireFormat()).toMatchObject([{ tags: ['0'] }, { tags: ["a 'funny' tag"] }])

    expect(w0.toString()).toEqual("'0' | 'a ''funny'' tag'")
  })
})

describe('tag automatic id extraction', () => {
  type FooWithId = {
    eventType: 'foo'
    fooId: string
  }

  const foo1: FooWithId = {
    eventType: 'foo',
    fooId: 'my-foo',
  }

  const foo2: FooWithId = {
    eventType: 'foo',
    fooId: 'second-foo',
  }

  const FooTag = Tag<FooWithId>('foo', foo => foo.fooId)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const BarTag = Tag<any>('bar')

  it('should extract id from events when applying', () => {
    expect(FooTag.apply(foo1).tags).toEqual(['foo', 'foo:my-foo'])
  })

  it('should extract id from multiple events when applying', () => {
    const evts = FooTag.apply(foo1, foo2)
    expect(evts[0].tags).toEqual(['foo', 'foo:my-foo'])
    expect(evts[1].tags).toEqual(['foo', 'foo:second-foo'])
  })

  it('should keep applying when ANDed with other tags', () => {
    expect(FooTag.and(BarTag).apply(foo1).tags).toEqual(['foo', 'foo:my-foo', 'bar'])
    expect(BarTag.and(FooTag).apply(foo1).tags).toEqual(['bar', 'foo', 'foo:my-foo'])
  })

  it('should not apply when already having a custom id specified', () => {
    const fooCustom = FooTag.withId('custom override')

    expect(fooCustom.apply(foo1).tags).toEqual(['foo', 'foo:custom override'])
    expect(fooCustom.and(BarTag).apply(foo1).tags).toEqual(['foo', 'foo:custom override', 'bar'])
  })
})
