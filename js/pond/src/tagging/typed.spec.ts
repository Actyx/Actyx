import { Tag, TypedTagQuery, matchAnyOf } from './typed'

const testTag = <T extends string>(tag: T) => Tag.create<T>(tag)

const tag0 = testTag('0')
const tag1 = testTag('1')

const tagA = testTag('A')
const tagB = testTag('B')

// '0' and '1' have no overlap, so only 'A' remains
export const q = matchAnyOf(tag0.and(tag1), tagA)

// Cannot omit event types that are actually included
// @ts-expect-error
export const q1: TypedTagQuery<'0' | '1'> = q

// It’s OK to manually give more types
export const q2: TypedTagQuery<'A' | 'more-types'> = q

// Tag that covers 3 types
const abcTag = Tag.create<'A' | 'B' | 'C'>('abc')

// Overlap is 'A'
export const w = tagA.and(abcTag)

// Does not turn into 'never'
// @ts-expect-error
export const w2: TypedTagQuery<never> = w

// Surface now is 'A', 'B', and 'C'
export const u = matchAnyOf(tagA, tagB, abcTag)

// Also covers 'C' now
// @ts-expect-error
export const u2: TypedTagQuery<'A' | 'B'> = u

// export const n: TypedTagQuery<'A'> = requireTag(
//   ...Tag.subTags(tagA, 'my-id', 'and-another-path-element-even'),
// )
