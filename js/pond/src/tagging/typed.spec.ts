import { matchAnyOf, Tag, TypedTagQuery } from './typed'

const testTag = <T extends string>(tag: T) => Tag.create<T>(tag)

const tag0 = testTag('0')
const tag1 = testTag('1')

const tagA = testTag('A')
const tagB = testTag('B')

// Tag that covers 3 types
const abcTag = Tag.create<'A' | 'B' | 'C'>('abc')

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = matchAnyOf(tag0.and(tag1), tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // @ts-expect-error
    const q1: TypedTagQuery<'0' | '1'> = q

    // point of this test is just to assert the TS-Error above
    expect(q1).toBeTruthy()
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
    expect(w2).toBeTruthy()
  })

  it('should union event types', () => {
    // Surface now is 'A', 'B', and 'C'
    const u = matchAnyOf(tagA, tagB.subSpace('some-id'), abcTag)

    // Also covers 'C' now
    // @ts-expect-error
    const u2: TypedTagQuery<'A' | 'B'> = u

    expect(u).toBeTruthy()
  })
})
