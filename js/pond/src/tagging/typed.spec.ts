import { Fish, FishId } from '../pond-v2-types'
import { Tag, Where } from './typed'

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

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = tag0.and(tag1).or(tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const q1: Where<'hello??'> = q

    // We use q1 so we don’t always get a TS error for unused variable.
    expect(q1.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['0', '1'] }, { type: 'intersection', tags: ['A'] }],
    })
  })

  it('should insist on types?', () => {
    // @ts-expect-error
    const q2: Where<'A' | 'more-types'> = q

    // Must use q2 to avoid TS error...
    expect(q2).toBeTruthy()
  })

  it('should preserve the common event type', () => {
    // Overlap is 'A'
    const w = tagA.and(abcTag)

    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const w2: Where<never> = w

    expect(w2.raw()).toMatchObject({
      type: 'intersection',
      tags: ['A', 'ABC'],
      onlyLocalEvents: false,
    })
  })

  it('should preserve local information', () => {
    // Overlap is 'A'
    const w = tagA.local().and(abcTag)

    expect(w.raw()).toMatchObject({
      type: 'intersection',
      tags: ['A', 'ABC'],
      onlyLocalEvents: true,
    })
  })

  it('should union event types ', () => {
    const u = tagA.or(tagB)

    expect(u.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['A'] }, { type: 'intersection', tags: ['B'] }],
    })
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

    expect(u.raw()).toMatchObject({
      type: 'union',
      tags: [
        { type: 'intersection', tags: ['A'], onlyLocalEvents: true },
        {
          type: 'intersection',
          tags: ['B', 'B:some-id'],
          onlyLocalEvents: false,
        },
        { type: 'intersection', tags: ['ABC'] },
      ],
    })
  })

  it('should require fish to implement onEvent that can handle all incoming events', () => {
    const fishWrong: Fish<undefined, A | B> = {
      onEvent: (state: undefined, _payload: A | B) => state,
      initialState: undefined,
      fishId: FishId.of('f', 'a', 0),

      // Expect error for too large subscription set
      // @ts-expect-error
      where: abcTag,
    }

    return fishWrong
  })

  it('should allow fish to handle more events than indicated by tags', () => {
    const fishRight: Fish<undefined, A | B | C | T0> = {
      onEvent: (state: undefined, _payload: A | B | C | T0) => state,
      initialState: undefined,
      fishId: FishId.of('f', 'a', 0),

      where: abcTag,
    }

    return fishRight
  })
})
