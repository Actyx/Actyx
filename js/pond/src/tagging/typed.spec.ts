import { matchAnyOf, Tag, TypedTagQuery } from './typed'

const testTag = <T extends string>(tag: T) => Tag<T>(tag)

const tag0 = testTag('0')
const tag1 = testTag('1')

const tagA = testTag('A')
const tagB = testTag('B')

// Tag that covers 3 types
const abcTag = Tag<'A' | 'B' | 'C'>('ABC')

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = matchAnyOf(tag0.and(tag1), tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const q1: TypedTagQuery<'hello??'> = q

    // We use q1 so we don’t always get a TS error for unused variable.
    expect(q1.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['0', '1'] }, { type: 'intersection', tags: ['A'] }],
    })
  })

  it('should allow expanding the type space manually', () => {
    // It’s OK to manually give more types
    const q2: TypedTagQuery<'A' | 'more-types'> = q

    // Must use q2 to avoid TS error...
    expect(q2).toBeTruthy()
  })

  it('should preserve the common event type', () => {
    // Overlap is 'A'
    const w = tagA.and(abcTag)

    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const w2: TypedTagQuery<never> = w

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

  it('should union event types', () => {
    // Surface now is 'A', 'B', and 'C'
    const u = matchAnyOf(tagA.local(), tagB.subSpace('some-id'), abcTag)

    // Also covers 'C' now
    // @ts-expect-error
    const u2: TypedTagQuery<'A' | 'B'> = u

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
})
