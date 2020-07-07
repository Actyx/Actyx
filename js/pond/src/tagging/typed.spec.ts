import { matchAnyOf, Tag, TypedTagQuery } from './typed'

const testTag = <T extends string>(tag: T) => Tag.create<T>(tag)

const tag0 = testTag('0')
const tag1 = testTag('1')

const tagA = testTag('A')
const tagB = testTag('B')

// Tag that covers 3 types
const abcTag = Tag.create<'A' | 'B' | 'C'>('ABC')

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = matchAnyOf(tag0.and(tag1), tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // @ts-expect-error
    const q1: TypedTagQuery<'0' | '1'> = q

    // point of this test is mostly to assert the TS-Error above
    expect(q1.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['0', '1'] }, { type: 'intersection', tags: ['A'] }],
    })
  })

  it('should allow expanding the type space manually', () => {
    // It’s OK to manually give more types
    const q2: TypedTagQuery<'A' | 'more-types'> = q

    // point of this test is just to assert the validity of the assignment
    expect(q2).toBeTruthy()
  })

  it('should preserve the common event type', () => {
    // Overlap is 'A'
    const w = tagA.and(abcTag)

    // @ts-expect-error
    const w2: TypedTagQuery<never> = w

    // point of this test is just to assert the ts error
    expect(w2.raw()).toMatchObject({
      type: 'intersection',
      tags: ['A', 'ABC'],
      onlyLocalEvents: false,
    })
  })

  it('should preserve local information', () => {
    // Overlap is 'A'
    const w = tagA.local().and(abcTag)

    // @ts-expect-error
    const w2: TypedTagQuery<never> = w

    // point of this test is just to assert the ts error
    expect(w2.raw()).toMatchObject({
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
