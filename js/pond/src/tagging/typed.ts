import { isString } from '..'
import { TagSubscription } from '../subscription'

const namedSubSpace = (rawTag: string, sub: string): string[] => {
  return [rawTag, rawTag + ':' + sub]
}

export interface TagsUnion<E> {
  /**
   * Add an alternative set we may also match. E.g. tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) will match:
   * Events with tag0; Events with both tag1 and tag2; Events with both tag1 and tag3.
   */
  or<E1>(tag: Tags<E1>): TagsUnion<E1 | E>

  /**
   * FOR INTERNAL USE. Convert to Actyx wire format.
   */
  toWireFormat(): ReadonlyArray<TagSubscription>

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-union'
}

// Must be interface, otherwise inferred (recursive) type gets very large.
export interface Tags<E> {
  /**
   * Add more tags to this requirement. E.g Tag<FooEvent>('foo').and(Tag<BarEvent>('bar')) will require both 'foo' and 'bar'.
   */
  and<E1>(tag: Tags<E1>): Tags<Extract<E1, E>>

  /**
   * Add an additional untyped tag to this requirement.
   * Since there is no associated type, the overall type cannot be constrained further.
   */
  and(tag: string): Tags<E>

  /**
   * Add an alternative set we may also match. E.g. Tag<FooEvent>('foo').or(Tag<BarEvent>('bar')) will match
   * each Event with at least 'foo' or 'bar'. Note that after the first `or` invocation you cannot `and` anymore,
   * so you have to nest the parts yourself: tag0.or(tag1.and(tag2)).or(tag1.and(tag3)) etc.
   */
  or<E1>(tag: Tags<E1>): TagsUnion<E1 | E>

  /**
   * The same requirement, but matching only Events emitted by the very node the code is run on.
   */
  local(): Tags<E>

  /**
   * FOR INTERNAL USE. Convert to Actyx wire format.
   */
  toWireFormat(): TagSubscription

  /**
   * Aggregated type of the Events which may be returned by the contained tags.
   */
  readonly _dataType?: E

  readonly type: 'typed-intersection'
}

export const Tags = <E>(...requiredTags: string[]): Tags<E> => req<E>(false, requiredTags)

export interface Tag<E> extends Tags<E> {
  // The underlying actual tag as pure string
  readonly rawTag: string

  withId(name: string): Tags<E>
}

export const Tag = <E>(rawTag: string): Tag<E> => ({
  rawTag,

  withId: (name: string) => req(false, namedSubSpace(rawTag, name)),

  ...req(false, [rawTag]),
})

/**
 * Typed expression for tag statements. The type `E` describes which events may be annotated with the included tags.
 */
export type Where<E> = TagsUnion<E> | Tags<E>

const req = <E>(onlyLocalEvents: boolean, rawTags: string[]): Tags<E> => {
  const r: Tags<E> = {
    and: <E1>(tag: Tags<E1> | string) => {
      if (isString(tag)) {
        return req<E>(onlyLocalEvents, [tag, ...rawTags])
      }

      const other = tag.toWireFormat()

      const local = onlyLocalEvents || !!other.local
      const tags = rawTags.concat(other.tags)

      return req<Extract<E1, E>>(local, tags)
    },

    or: <E1>(other: Tags<E1>) => {
      return union<E1 | E>([req(onlyLocalEvents, rawTags), other])
    },

    local: () => req<E>(true, rawTags),

    type: 'typed-intersection',

    toWireFormat: () => ({
      tags: [...rawTags],

      local: onlyLocalEvents,
    }),
  }

  return r
}

const union = <E>(sets: Tags<unknown>[]): TagsUnion<E> => {
  return {
    type: 'typed-union',

    or: <E1>(other: Tags<E1>) => {
      return union<E1 | E>([...sets, other])
    },

    toWireFormat: () => sets.map(x => x.toWireFormat()),
  }
}

export const allEvents: Tags<unknown> = req(false, [])
export const noEvents: TagsUnion<never> = union([])
