import { Tag, TypedTagQuery } from './typed'

const testTag = <T extends string>(tag: T) => Tag.mk<T>(tag)

const tag0 = testTag('0')
const tag1 = testTag('1')

const tagA = testTag('A')
const tagB = testTag('B')

const { requireTag, matchAnyOf } = TypedTagQuery

// '0' and '1' have no overlap, so only 'A' remains
export const q = matchAnyOf(requireTag(tag0).and(tag1), requireTag(tagA))

// Cannot omit event types that are actually included
// @ts-expect-error
export const q1: TypedTagQuery<'0' | '1'> = q

// It’s OK to manually give more types
export const q2: TypedTagQuery<'A' | 'more-types'> = q

// Tag that covers 3 types
const abcTag = Tag.mk<'A' | 'B' | 'C'>('abc')

// Overlap is 'A'
export const w = requireTag(tagA).and(abcTag)

// Does not turn into 'never'
// @ts-expect-error
export const w2: TypedTagQuery<never> = w

// Surface now is 'A', 'B', and 'C'
export const u = matchAnyOf(requireTag(tagA), requireTag(tagB), requireTag(abcTag))

// Also covers 'C' now
// @ts-expect-error
export const u2: TypedTagQuery<'A' | 'B'> = u
